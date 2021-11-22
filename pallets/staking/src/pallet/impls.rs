use crate::{
    log, ActiveEra, ActiveEraInfo, BondedEras, Config, CounterForValidators, CurrentEra,
    CurrentPlannedSession, EraIndex, ErasStakers, ErasStartSessionIndex, ErasTotalStake, Event,
    Exposure, ForceEra, Forcing, IndividualExposure, Ledger, Pallet, SessionInterface,
    StakingLedger, Validators, Value,
};

use crate::weights::WeightInfo;
use frame_election_provider_support::{
    data_provider, ElectionDataProvider, ElectionProvider, Supports, VoteWeight,
};
use frame_support::dispatch::{Vec, Weight};
use frame_support::pallet_prelude::DispatchClass;
use frame_support::sp_runtime::traits::{Bounded, Zero};
use frame_support::traits::{CurrencyToVote, EstimateNextNewSession};
use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::Saturating;
use sp_staking::SessionIndex;
use sp_std::vec;

impl<T: Config> Pallet<T> {
    /// The total balance that can be slashed from a stash account as of right now.
    pub fn slashable_balance_of(stash: &T::AccountId) -> Value {
        // Weight note: consider making the stake accessible through stash.
        Self::bonded(stash).and_then(Self::ledger).map(|l| l.total).unwrap_or_default()
    }

    /// Internal impl of [`Self::slashable_balance_of`] that returns [`VoteWeight`].
    pub fn slashable_balance_of_vote_weight(stash: &T::AccountId, issuance: Value) -> VoteWeight {
        T::CurrencyToVote::to_vote(Self::slashable_balance_of(stash), issuance)
    }

    /// Returns a closure around `slashable_balance_of_vote_weight` that can be passed around.
    ///
    /// This prevents call sites from repeatedly requesting `total_issuance` from backend. But it is
    /// important to be only used while the total issuance is not changing.
    pub fn weight_of(who: &T::AccountId) -> VoteWeight {
        // TODO: this should use the total issuance
        let overall_stake = Self::overall_stake_value();

        Self::slashable_balance_of_vote_weight(who, overall_stake)
    }

    // TODO: this should be using the total issuance
    pub fn overall_stake_value() -> Value {
        let mut total: Value = 0;
        Ledger::<T>::iter_values().for_each(|ledger| {
            if ledger.unlocking_era.is_none() {
                total = total.saturating_add(ledger.total);
            }
        });

        total
    }

    /// add the ledger for a controller.
    pub(crate) fn add_ledger(stash: T::AccountId, controller: T::AccountId, value: u128) {
        let current_era = CurrentEra::<T>::get().unwrap_or(0);
        let history_depth = Self::history_depth();
        let last_reward_era = current_era.saturating_sub(history_depth);

        let ledger = StakingLedger {
            stash: stash.clone(),
            total: value,
            unlocking_era: None,
            claimed_rewards: (last_reward_era..current_era).collect(),
        };
        <Ledger<T>>::insert(controller, ledger);
        <Pallet<T>>::deposit_event(Event::<T>::Bonded(stash, value));
    }

    /// update the ledger for a controller
    pub(crate) fn update_ledger(
        stash: T::AccountId,
        controller: T::AccountId,
        value: u128,
        ledger: &mut StakingLedger<T::AccountId>,
    ) {
        ledger.total += value;
        <Ledger<T>>::insert(controller, ledger);
        <Pallet<T>>::deposit_event(Event::<T>::Bonded(stash, value));
    }

    /// Get all of the voters that are eligible for the npos election.
    ///
    /// `maybe_max_len` can imposes a cap on the number of voters returned; First all the validator
    /// are included in no particular order, then remainder is taken from the nominators, as
    /// returned by [`Config::SortedListProvider`].
    ///
    /// This will use nominators, and all the validators will inject a self vote.
    ///
    /// This function is self-weighing as [`DispatchClass::Mandatory`].
    ///
    /// ### Slashing
    ///
    /// All nominations that have been submitted before the last non-zero slash of the validator are
    /// auto-chilled, but still count towards the limit imposed by `maybe_max_len`.
    pub fn get_npos_voters(
        maybe_max_len: Option<usize>,
    ) -> Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)> {
        let max_allowed_len = {
            let validator_count = CounterForValidators::<T>::get() as usize;
            maybe_max_len.unwrap_or(validator_count).min(validator_count)
        };

        let mut all_voters = Vec::<_>::with_capacity(max_allowed_len);

        // grab all validators in no particular order, capped by the maximum allowed length.
        let mut validators_taken = 0u32;
        for (validator, _) in <Validators<T>>::iter().take(max_allowed_len) {
            // Append self vote.
            let self_vote = (
                validator.clone(),
                Self::weight_of(&validator),
                vec![validator.clone()],
            );
            all_voters.push(self_vote);
            validators_taken.saturating_inc();
        }

        // all_voters should have not re-allocated.
        debug_assert!(all_voters.capacity() == max_allowed_len);

        Self::register_weight(T::WeightInfo::get_npos_voters(validators_taken, 0 as u32));

        log!(
            info,
            "generated {} npos voters, {} from validators",
            all_voters.len(),
            validators_taken,
        );
        all_voters
    }

    /// Get the targets for an upcoming npos election.
    ///
    /// This function is self-weighing as [`DispatchClass::Mandatory`].
    pub fn get_npos_targets() -> Vec<T::AccountId> {
        let mut validator_count = 0u32;
        let targets = Validators::<T>::iter()
            .map(|(v, _)| {
                validator_count.saturating_inc();
                v
            })
            .collect::<Vec<_>>();

        Self::register_weight(T::WeightInfo::get_npos_targets(validator_count));

        targets
    }

    /// This function will add a validator to the `Validators` storage map, and keep track of the
    /// `CounterForValidators`.
    ///
    /// NOTE: you must ALWAYS use this function to add a validator to the system. Any access to
    /// `Validators`, its counter, or `VoterList` outside of this function is almost certainly
    /// wrong.
    pub fn do_add_validator(who: &T::AccountId, controller: &T::AccountId) {
        CounterForValidators::<T>::mutate(|x| x.saturating_inc());
        Validators::<T>::insert(who, controller);
    }

    /// Plan a new session potentially trigger a new era.
    fn new_session(session_index: SessionIndex, is_genesis: bool) -> Option<Vec<T::AccountId>> {
        if let Some(current_era) = Self::current_era() {
            // Initial era has been set.
            let current_era_start_session_index = Self::eras_start_session_index(current_era)
                .unwrap_or_else(|| {
                    frame_support::print("Error: start_session_index must be set for current_era");
                    0
                });

            let era_length =
                session_index.checked_sub(current_era_start_session_index).unwrap_or(0); // Must never happen.

            match ForceEra::<T>::get() {
                // Will be set to `NotForcing` again if a new era has been triggered.
                Forcing::ForceNew => (),
                // Short circuit to `try_trigger_new_era`.
                Forcing::ForceAlways => (),
                // Only go to `try_trigger_new_era` if deadline reached.
                Forcing::NotForcing if era_length >= T::SessionsPerEra::get() => (),
                _ => {
                    // Either `Forcing::ForceNone`,
                    // or `Forcing::NotForcing if era_length >= T::SessionsPerEra::get()`.
                    return None;
                }
            }

            // New era.
            let maybe_new_era_validators = Self::try_trigger_new_era(session_index, is_genesis);
            if maybe_new_era_validators.is_some()
                && matches!(ForceEra::<T>::get(), Forcing::ForceNew)
            {
                ForceEra::<T>::put(Forcing::NotForcing);
            }

            maybe_new_era_validators
        } else {
            // Set initial era.
            log!(debug, "Starting the first era.");
            Self::try_trigger_new_era(session_index, is_genesis)
        }
    }

    /// Start a session potentially starting an era.
    fn start_session(start_session: SessionIndex) {
        let next_active_era = Self::active_era().map(|e| e.index + 1).unwrap_or(0);
        // This is only `Some` when current era has already progressed to the next era, while the
        // active era is one behind (i.e. in the *last session of the active era*, or *first session
        // of the new current era*, depending on how you look at it).
        if let Some(next_active_era_start_session_index) =
            Self::eras_start_session_index(next_active_era)
        {
            if next_active_era_start_session_index == start_session {
                Self::start_era(start_session);
            } else if next_active_era_start_session_index < start_session {
                // This arm should never happen, but better handle it than to stall the staking
                // pallet.
                frame_support::print("Warning: A session appears to have been skipped.");
                Self::start_era(start_session);
            }
        }
    }

    /// End a session potentially ending an era.
    fn end_session(session_index: SessionIndex) {
        if let Some(active_era) = Self::active_era() {
            if let Some(next_active_era_start_session_index) =
                Self::eras_start_session_index(active_era.index + 1)
            {
                if next_active_era_start_session_index == session_index + 1 {
                    // Self::end_era(active_era, session_index);
                }
            }
        }
    }

    ///
    /// * Increment `active_era.index`,
    /// * reset `active_era.start`,
    /// * update `BondedEras` and apply slashes.
    fn start_era(start_session: SessionIndex) {
        let active_era = ActiveEra::<T>::mutate(|active_era| {
            let new_index = active_era.as_ref().map(|info| info.index + 1).unwrap_or(0);
            *active_era = Some(ActiveEraInfo {
                index: new_index,
                // Set new active era start in next `on_finalize`. To guarantee usage of `Time`
                start: None,
            });
            new_index
        });

        let bonding_duration = T::BondingDuration::get();

        BondedEras::<T>::mutate(|bonded| {
            bonded.push((active_era, start_session));

            if active_era > bonding_duration {
                if let Some(&(_, first_session)) = bonded.first() {
                    T::SessionInterface::prune_historical_up_to(first_session);
                }
            }
        });
    }

    /// Plan a new era.
    ///
    /// * Bump the current era storage (which holds the latest planned era).
    /// * Store start session index for the new planned era.
    /// * Clean old era information.
    /// * Store staking information for the new planned era
    ///
    /// Returns the new validator set.
    pub fn trigger_new_era(
        start_session_index: SessionIndex,
        exposures: Vec<(T::AccountId, Exposure<T::AccountId>)>,
    ) -> Vec<T::AccountId> {
        // Increment or set current era.
        let new_planned_era = CurrentEra::<T>::mutate(|s| {
            *s = Some(s.map(|s| s + 1).unwrap_or(0));
            s.unwrap()
        });
        ErasStartSessionIndex::<T>::insert(&new_planned_era, &start_session_index);

        // Clean old era information.
        if let Some(old_era) = new_planned_era.checked_sub(Self::history_depth() + 1) {
            Self::clear_era_information(old_era);
        }

        // Set staking information for the new era.
        Self::store_stakers_info(exposures, new_planned_era)
    }

    /// Potentially plan a new era.
    ///
    /// Get election result from `T::ElectionProvider`.
    /// In case election result has more than [`MinimumValidatorCount`] validator trigger a new era.
    ///
    /// In case a new era is planned, the new validator set is returned.
    pub(crate) fn try_trigger_new_era(
        start_session_index: SessionIndex,
        is_genesis: bool,
    ) -> Option<Vec<T::AccountId>> {
        let election_result = if is_genesis {
            T::GenesisElectionProvider::elect().map_err(|e| {
                log!(warn, "genesis election provider failed due to {:?}", e);
                Self::deposit_event(Event::StakingElectionFailed);
            })
        } else {
            T::ElectionProvider::elect().map_err(|e| {
                log!(warn, "election provider failed due to {:?}", e);
                Self::deposit_event(Event::StakingElectionFailed);
            })
        }
        .ok()?;

        let exposures = Self::collect_exposures(election_result);

        if (exposures.len() as u32) < Self::minimum_validator_count().max(1) {
            // Session will panic if we ever return an empty validator set, thus max(1) ^^.
            match CurrentEra::<T>::get() {
                None => {
                    // The initial era is allowed to have no exposures.
                    // In this case the SessionManager is expected to choose a sensible validator
                    // set.
                    // TODO: this should be simplified #8911
                    CurrentEra::<T>::put(0);
                    ErasStartSessionIndex::<T>::insert(&0, &start_session_index);
                }
                Some(current_era) if current_era > 0 => log!(
                    warn,
                    "chain does not have enough staking candidates to operate for era {:?} ({} \
					elected, minimum is {})",
                    CurrentEra::<T>::get().unwrap_or(0),
                    exposures.len(),
                    Self::minimum_validator_count(),
                ),
                _ => (),
            }
            Self::deposit_event(Event::StakingElectionFailed);
            return None;
        }

        Self::deposit_event(Event::StakersElected);
        Some(Self::trigger_new_era(start_session_index, exposures))
    }

    /// Process the output of the election.
    ///
    /// Store staking information for the new planned era
    pub fn store_stakers_info(
        exposures: Vec<(T::AccountId, Exposure<T::AccountId>)>,
        new_planned_era: EraIndex,
    ) -> Vec<T::AccountId> {
        let elected_stashes = exposures.iter().cloned().map(|(x, _)| x).collect::<Vec<_>>();

        // Populate stakers, exposures, and the snapshot of validator prefs.
        let mut total_stake: Value = 0;
        exposures.into_iter().for_each(|(stash, stake)| {
            total_stake = total_stake.saturating_add(stake.total);
            <ErasStakers<T>>::insert(new_planned_era, &stash, &stake);
        });

        // Insert current era staking information
        <ErasTotalStake<T>>::insert(&new_planned_era, total_stake);

        if new_planned_era > 0 {
            log!(
                info,
                "new validator set of size {:?} has been processed for era {:?}",
                elected_stashes.len(),
                new_planned_era,
            );
        }

        elected_stashes
    }

    /// Consume a set of [`Supports`] from [`sp_npos_elections`] and collect them into a
    /// [`(validator, weight)`].
    pub(crate) fn collect_exposures(
        supports: Supports<T::AccountId>,
    ) -> Vec<(T::AccountId, Exposure<T::AccountId>)> {
        // TODO: In substrate, the total issuance is used to extract the weight of the stake.
        // For now, the total staked will of an account will determine the vote.
        let overall_stake = Self::overall_stake_value();
        let to_currency = |e: frame_election_provider_support::ExtendedBalance| {
            T::CurrencyToVote::to_currency(e, overall_stake)
        };

        supports
            .into_iter()
            .map(|(validator, support)| {
                // Build `struct exposure` from `support`.
                let mut others = Vec::with_capacity(support.voters.len());
                let mut own: Value = 0;
                let mut total: Value = 0;

                support.voters.into_iter().for_each(|(nominator, stake)| {
                    let stake = to_currency(stake);

                    if nominator == validator {
                        log!(info, "voting for myself: {:?}", validator);
                        own = own.saturating_add(stake);
                    } else {
                        log!(info, "account {:?} votes for {:?}", nominator, validator);
                        others.push(IndividualExposure {
                            who: nominator,
                            value: stake,
                        });
                    }
                    total = total.saturating_add(stake);
                });

                let exposure = Exposure { total, own, others };

                (validator, exposure)
            })
            .collect()
    }

    /// Clear all era information for given era.
    pub(crate) fn clear_era_information(era_index: EraIndex) {
        <ErasStakers<T>>::remove_prefix(era_index, None);
        <ErasTotalStake<T>>::remove(era_index);
        <ErasStartSessionIndex<T>>::remove(era_index);
    }

    /// Register some amount of weight directly with the system pallet.
    ///
    /// This is always mandatory weight.
    fn register_weight(weight: Weight) {
        <frame_system::Pallet<T>>::register_extra_weight_unchecked(
            weight,
            DispatchClass::Mandatory,
        );
    }
}

impl<T: Config> ElectionDataProvider<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
    const MAXIMUM_VOTES_PER_VOTER: u32 = 0;

    fn desired_targets() -> data_provider::Result<u32> {
        Self::register_weight(T::DbWeight::get().reads(1));
        Ok(Self::validator_count())
    }

    fn voters(
        maybe_max_len: Option<usize>,
    ) -> data_provider::Result<Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)>> {
        debug_assert!(<Validators<T>>::iter().count() as u32 == CounterForValidators::<T>::get());

        // This can never fail -- if `maybe_max_len` is `Some(_)` we handle it.
        let voters = Self::get_npos_voters(maybe_max_len);
        debug_assert!(maybe_max_len.map_or(true, |max| voters.len() <= max));

        Ok(voters)
    }

    fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<T::AccountId>> {
        let target_count = CounterForValidators::<T>::get();

        // We can't handle this case yet -- return an error.
        if maybe_max_len.map_or(false, |max_len| target_count > max_len as u32) {
            return Err("Target snapshot too big");
        }

        Ok(Self::get_npos_targets())
    }

    fn next_election_prediction(now: T::BlockNumber) -> T::BlockNumber {
        let current_era = Self::current_era().unwrap_or(0);
        let current_session = Self::current_planned_session();
        let current_era_start_session_index =
            Self::eras_start_session_index(current_era).unwrap_or(0);
        // Number of session in the current era or the maximum session per era if reached.
        let era_progress = current_session
            .saturating_sub(current_era_start_session_index)
            .min(T::SessionsPerEra::get());

        let until_this_session_end = T::NextNewSession::estimate_next_new_session(now)
            .0
            .unwrap_or_default()
            .saturating_sub(now);

        let session_length = T::NextNewSession::average_session_length();

        let sessions_left: T::BlockNumber = match ForceEra::<T>::get() {
            Forcing::ForceNone => Bounded::max_value(),
            Forcing::ForceNew | Forcing::ForceAlways => Zero::zero(),
            Forcing::NotForcing if era_progress >= T::SessionsPerEra::get() => Zero::zero(),
            Forcing::NotForcing => T::SessionsPerEra::get()
                .saturating_sub(era_progress)
                // One session is computed in this_session_end.
                .saturating_sub(1)
                .into(),
        };

        now.saturating_add(
            until_this_session_end.saturating_add(sessions_left.saturating_mul(session_length)),
        )
    }
}

/// In this implementation `new_session(session)` must be called before `end_session(session-1)`
/// i.e. the new session must be planned before the ending of the previous session.
///
/// Once the first new_session is planned, all session must start and then end in order, though
/// some session can lag in between the newest session planned and the latest session started.
impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
    fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        log!(trace, "planning new session {}", new_index);
        CurrentPlannedSession::<T>::put(new_index);
        Self::new_session(new_index, false)
    }
    fn new_session_genesis(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        log!(trace, "planning new session {} at genesis", new_index);
        CurrentPlannedSession::<T>::put(new_index);
        Self::new_session(new_index, true)
    }
    fn end_session(end_index: SessionIndex) {
        log!(trace, "ending session {}", end_index);
        Self::end_session(end_index)
    }
    fn start_session(start_index: SessionIndex) {
        log!(trace, "starting session {}", start_index);
        Self::start_session(start_index)
    }
}

impl<T: Config> pallet_session::historical::SessionManager<T::AccountId, Exposure<T::AccountId>>
    for Pallet<T>
{
    fn new_session(new_index: SessionIndex) -> Option<Vec<(T::AccountId, Exposure<T::AccountId>)>> {
        <Self as pallet_session::SessionManager<_>>::new_session(new_index).map(|validators| {
            let current_era = Self::current_era()
                // Must be some as a new era has been created.
                .unwrap_or(0);

            validators
                .into_iter()
                .map(|v| {
                    let exposure = Self::eras_stakers(current_era, &v);
                    (v, exposure)
                })
                .collect()
        })
    }
    fn new_session_genesis(
        new_index: SessionIndex,
    ) -> Option<Vec<(T::AccountId, Exposure<T::AccountId>)>> {
        <Self as pallet_session::SessionManager<_>>::new_session_genesis(new_index).map(
            |validators| {
                let current_era = Self::current_era()
                    // Must be some as a new era has been created.
                    .unwrap_or(0);

                validators
                    .into_iter()
                    .map(|v| {
                        let exposure = Self::eras_stakers(current_era, &v);
                        (v, exposure)
                    })
                    .collect()
            },
        )
    }
    fn start_session(start_index: SessionIndex) {
        <Self as pallet_session::SessionManager<_>>::start_session(start_index)
    }
    fn end_session(end_index: SessionIndex) {
        <Self as pallet_session::SessionManager<_>>::end_session(end_index)
    }
}