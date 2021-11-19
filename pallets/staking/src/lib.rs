#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

mod locking;
mod pallet;
pub mod weights;

use codec::{Decode, Encode};
use frame_support::dispatch::DispatchResult;
use frame_support::sp_runtime::traits::Convert;
pub use locking::*;
pub use pallet::{pallet::*, *};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_staking::SessionIndex;
use sp_std::vec::Vec;

pub(crate) const LOG_TARGET: &'static str = "runtime::staking";
pub type Value = u128;

// syntactic sugar for logging.
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: crate::LOG_TARGET,
			concat!("[{:?}] 💸 ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

/// Information regarding the active era (era in used in session).
#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ActiveEraInfo {
    /// Index of era.
    pub index: EraIndex,
    /// Moment of start expressed as millisecond from `$UNIX_EPOCH`.
    ///
    /// Start can be none if start hasn't been set for the era yet,
    /// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
    start: Option<u64>,
}

/// Indicates the initial status of the staker.
#[derive(RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum StakerStatus {
    /// Chilling.
    Idle,
    /// Declared desire in validating or already participating in it.
    Validator,
}

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

/// The ledger of a (bonded) stash.
#[cfg_attr(feature = "runtime-benchmarks", derive(Default))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct StakingLedger<AccountId> {
    /// The stash account whose balance is actually locked and at stake.
    pub stash: AccountId,
    /// The total amount of the stash's balance that we are currently accounting for.
    pub total: Value,
    /// Era number at which point the total balance may eventually
    /// be transfered out of the stash.
    pub unlocking_era: Option<EraIndex>,
    /// List of eras for which the stakers behind a validator have claimed rewards. Only updated
    /// for validators.
    pub claimed_rewards: Vec<EraIndex>,
}

impl<AccountId> StakingLedger<AccountId> {
    /// Re-bond funds that were scheduled for unlocking.
    fn rebond(mut self) -> Self {
        self.unlocking_era = None;
        self
    }
}

/// The amount of exposure (to slashing) than an individual nominator has.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct IndividualExposure<AccountId> {
    /// The stash account of the nominator in question.
    pub who: AccountId,
    /// Amount of funds exposed.
    pub value: Value,
}

/// A snapshot of the stake backing a single validator in the system.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, Default, RuntimeDebug, TypeInfo,
)]
pub struct Exposure<AccountId> {
    /// The total balance backing this validator.
    pub total: Value,
    /// The validator's own stash that is exposed.
    pub own: Value,
    /// The portions of nominators stashes that are exposed.
    pub others: Vec<IndividualExposure<AccountId>>,
}

/// Means for interacting with a specialized version of the `session` trait.
///
/// This is needed because `Staking` sets the `ValidatorIdOf` of the `pallet_session::Config`
pub trait SessionInterface<AccountId>: frame_system::Config {
    /// Disable a given validator by stash ID.
    ///
    /// Returns `true` if new era should be forced at the end of this session.
    /// This allows preventing a situation where there is too many validators
    /// disabled and block production stalls.
    fn disable_validator(validator: &AccountId) -> Result<bool, ()>;
    /// Get the validators from session.
    fn validators() -> Vec<AccountId>;
    /// Prune historical session tries up to but not including the given index.
    fn prune_historical_up_to(up_to: SessionIndex);
}

impl<T: Config> SessionInterface<<T as frame_system::Config>::AccountId> for T
where
    T: pallet_session::Config<ValidatorId = <T as frame_system::Config>::AccountId>,
    T: pallet_session::historical::Config<
        FullIdentification = Exposure<<T as frame_system::Config>::AccountId>,
        FullIdentificationOf = ExposureOf<T>,
    >,
    T::SessionHandler: pallet_session::SessionHandler<<T as frame_system::Config>::AccountId>,
    T::SessionManager: pallet_session::SessionManager<<T as frame_system::Config>::AccountId>,
    T::ValidatorIdOf: Convert<
        <T as frame_system::Config>::AccountId,
        Option<<T as frame_system::Config>::AccountId>,
    >,
{
    fn disable_validator(validator: &<T as frame_system::Config>::AccountId) -> Result<bool, ()> {
        <pallet_session::Pallet<T>>::disable(validator)
    }

    fn validators() -> Vec<<T as frame_system::Config>::AccountId> {
        <pallet_session::Pallet<T>>::validators()
    }

    fn prune_historical_up_to(up_to: SessionIndex) {
        <pallet_session::historical::Pallet<T>>::prune_up_to(up_to);
    }
}

pub trait SettingSessionKey<AccountId> {
    fn can_decode_session_keys(session_key: &Vec<u8>) -> bool;
    fn set_session_keys(controller: AccountId, session_keys: &Vec<u8>) -> DispatchResult;
}

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Forcing {
    /// Not forcing anything - just let whatever happen.
    NotForcing,
    /// Force a new era, then reset to `NotForcing` as soon as it is done.
    /// Note that this will force to trigger an election until a new era is triggered, if the
    /// election failed, the next session end will trigger a new election again, until success.
    ForceNew,
    /// Avoid a new era indefinitely.
    ForceNone,
    /// Force a new era at the end of all sessions indefinitely.
    ForceAlways,
}

impl Default for Forcing {
    fn default() -> Self {
        Forcing::NotForcing
    }
}

/// A `Convert` implementation that finds the stash of the given controller account,
/// if any.
pub struct StashOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for StashOf<T> {
    fn convert(controller: T::AccountId) -> Option<T::AccountId> {
        <Pallet<T>>::ledger(&controller).map(|l| l.stash)
    }
}

/// A typed conversion from stash account ID to the active exposure of nominators
/// on that account.
///
/// Active exposure is the exposure of the validator set currently validating, i.e. in
/// `active_era`. It can differ from the latest planned exposure in `current_era`.
pub struct ExposureOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Convert<T::AccountId, Option<Exposure<T::AccountId>>> for ExposureOf<T> {
    fn convert(validator: T::AccountId) -> Option<Exposure<T::AccountId>> {
        <Pallet<T>>::active_era()
            .map(|active_era| <Pallet<T>>::eras_stakers(active_era.index, &validator))
    }
}
