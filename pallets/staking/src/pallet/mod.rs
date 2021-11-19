mod impls;

pub const MAX_UNLOCKING_CHUNKS: usize = 32;

pub use impls::*;

#[frame_support::pallet]
pub mod pallet {
    use crate::{
        locking::Balance, log, weights::WeightInfo, ActiveEraInfo, EraIndex, Exposure, Forcing,
        SessionInterface, SettingSessionKey, StakingLedger, Value,
    };
    use frame_support::traits::CurrencyToVote;
    use frame_support::{
        pallet_prelude::*,
        traits::{EstimateNextNewSession, Get, UnixTime},
        weights::Weight,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{SaturatedConversion, StaticLookup};
    use sp_staking::SessionIndex;
    use sp_std::{convert::From, prelude::*};

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Balance: Balance<Self::AccountId>;

        type SettingSessionKey: SettingSessionKey<Self::AccountId>;

        /// Time used for computing era duration.
        ///
        /// It is guaranteed to start being called from the first `on_finalize`. Thus value at
        /// genesis is not used.
        type UnixTime: UnixTime;

        /// Convert a balance into a number used for election calculation. This must fit into a
        /// `u64` but is allowed to be sensibly lossy. The `u64` is used to communicate with the
        /// [`sp_npos_elections`] crate which accepts u64 numbers and does operations in 128.
        /// Consequently, the backward convert is used convert the u128s from sp-elections back to a
        /// [`Value`].
        type CurrencyToVote: CurrencyToVote<Value>;

        /// Something that provides the election functionality.
        type ElectionProvider: frame_election_provider_support::ElectionProvider<
            Self::AccountId,
            Self::BlockNumber,
            // we only accept an election provider that has staking as data provider.
            DataProvider = Pallet<Self>,
        >;

        /// Something that provides the election functionality at genesis.
        type GenesisElectionProvider: frame_election_provider_support::ElectionProvider<
            Self::AccountId,
            Self::BlockNumber,
            DataProvider = Pallet<Self>,
        >;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Number of sessions per era.
        #[pallet::constant]
        type SessionsPerEra: Get<SessionIndex>;

        /// Number of eras that staked funds must remain bonded for.
        #[pallet::constant]
        type BondingDuration: Get<EraIndex>;

        /// Interface for interacting with a session pallet.
        type SessionInterface: SessionInterface<Self::AccountId>;

        /// Something that can estimate the next session change, accurately or as a best effort
        /// guess.
        type NextNewSession: EstimateNextNewSession<Self::BlockNumber>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::type_value]
    pub(crate) fn HistoryDepthOnEmpty() -> u32 {
        84u32
    }

    /// Number of eras to keep in history.
    ///
    /// Information is kept for eras in `[current_era - history_depth; current_era]`.
    ///
    /// Must be more than the number of eras delayed by session otherwise. I.e. active era must
    /// always be in history. I.e. `active_era > current_era - history_depth` must be
    /// guaranteed.
    #[pallet::storage]
    #[pallet::getter(fn history_depth)]
    pub(crate) type HistoryDepth<T> = StorageValue<_, u32, ValueQuery, HistoryDepthOnEmpty>;

    /// The ideal number of staking participants.
    #[pallet::storage]
    #[pallet::getter(fn validator_count)]
    pub type ValidatorCount<T> = StorageValue<_, u32, ValueQuery>;

    /// Minimum number of staking participants before emergency conditions are imposed.
    #[pallet::storage]
    #[pallet::getter(fn minimum_validator_count)]
    pub type MinimumValidatorCount<T> = StorageValue<_, u32, ValueQuery>;

    /// Any validators that may never be slashed or forcibly kicked. It's a Vec since they're
    /// easy to initialize and the performance hit is minimal (we expect no more than four
    /// invulnerables) and restricted to testnets.
    #[pallet::storage]
    #[pallet::getter(fn invulnerables)]
    pub type Invulnerables<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    /// Map from all locked "stash" accounts to the controller account.
    #[pallet::storage]
    #[pallet::getter(fn bonded)]
    pub type Bonded<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

    #[pallet::storage]
    pub type MinValidatorBond<T: Config> = StorageValue<_, Value, ValueQuery>;

    /// Map from all (unlocked) "controller" accounts to the info regarding the staking.
    #[pallet::storage]
    #[pallet::getter(fn ledger)]
    pub type Ledger<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, StakingLedger<T::AccountId>>;

    /// The map from (wannabe) validator stash key to the preferences of that validator.
    ///
    /// When updating this storage item, you must also update the `CounterForValidators`.
    #[pallet::storage]
    #[pallet::getter(fn validators)]
    pub type Validators<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, T::AccountId, ValueQuery>;

    /// A tracker to keep count of the number of items in the `Bonded` map.
    #[pallet::storage]
    pub type CounterForValidators<T> = StorageValue<_, u32, ValueQuery>;

    /// The maximum validator count before we stop allowing new validators to join.
    ///
    /// When this value is not set, no limits are enforced.
    #[pallet::storage]
    pub type MaxValidatorsCount<T> = StorageValue<_, u32, OptionQuery>;

    /// The current era index.
    ///
    /// This is the latest planned era, depending on how the Session pallet queues the validator
    /// set, it might be active or not.
    #[pallet::storage]
    #[pallet::getter(fn current_era)]
    pub type CurrentEra<T> = StorageValue<_, EraIndex>;

    /// The active era information, it holds index and start.
    ///
    /// The active era is the era being currently rewarded. Validator set of this era must be
    /// equal to [`SessionInterface::validators`].
    #[pallet::storage]
    #[pallet::getter(fn active_era)]
    pub type ActiveEra<T> = StorageValue<_, ActiveEraInfo>;

    /// The session index at which the era start for the last `HISTORY_DEPTH` eras.
    ///
    /// Note: This tracks the starting session (i.e. session index when era start being active)
    /// for the eras in `[CurrentEra - HISTORY_DEPTH, CurrentEra]`.
    #[pallet::storage]
    #[pallet::getter(fn eras_start_session_index)]
    pub type ErasStartSessionIndex<T> = StorageMap<_, Twox64Concat, EraIndex, SessionIndex>;

    /// Exposure of validator at era.
    ///
    /// This is keyed first by the era index to allow bulk deletion and then the stash account.
    ///
    /// Is it removed after `HISTORY_DEPTH` eras.
    /// If stakers hasn't been set or has been removed then empty exposure is returned.
    #[pallet::storage]
    #[pallet::getter(fn eras_stakers)]
    pub type ErasStakers<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        EraIndex,
        Twox64Concat,
        T::AccountId,
        Exposure<T::AccountId>,
        ValueQuery,
    >;

    /// The total amount staked for the last `HISTORY_DEPTH` eras.
    /// If total hasn't been set or has been removed then 0 stake is returned.
    #[pallet::storage]
    #[pallet::getter(fn eras_total_stake)]
    pub type ErasTotalStake<T: Config> = StorageMap<_, Twox64Concat, EraIndex, Value, ValueQuery>;

    /// Mode of era forcing.
    #[pallet::storage]
    #[pallet::getter(fn force_era)]
    pub type ForceEra<T> = StorageValue<_, Forcing, ValueQuery>;

    /// A mapping from still-bonded eras to the first session index of that era.
    ///
    /// Must contains information for eras for the range:
    /// `[active_era - bounding_duration; active_era]`
    #[pallet::storage]
    pub(crate) type BondedEras<T: Config> =
        StorageValue<_, Vec<(EraIndex, SessionIndex)>, ValueQuery>;

    /// The last planned session scheduled by the session pallet.
    ///
    /// This is basically in sync with the call to [`pallet_session::SessionManager::new_session`].
    #[pallet::storage]
    #[pallet::getter(fn current_planned_session)]
    pub type CurrentPlannedSession<T> = StorageValue<_, SessionIndex, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub history_depth: u32,
        pub validator_count: u32,
        pub minimum_validator_count: u32,
        pub invulnerables: Vec<T::AccountId>,
        pub force_era: Forcing,
        pub stakers: Vec<(T::AccountId, T::AccountId, Value, crate::StakerStatus)>,
        pub min_validator_bond: Value,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            GenesisConfig {
                history_depth: 84u32,
                validator_count: Default::default(),
                minimum_validator_count: Default::default(),
                invulnerables: Default::default(),
                force_era: Default::default(),
                stakers: Default::default(),
                min_validator_bond: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            HistoryDepth::<T>::put(self.history_depth);
            ValidatorCount::<T>::put(self.validator_count);
            MinimumValidatorCount::<T>::put(self.minimum_validator_count);
            Invulnerables::<T>::put(&self.invulnerables);
            ForceEra::<T>::put(self.force_era);
            MinValidatorBond::<T>::put(self.min_validator_bond);

            for &(ref stash, ref controller, balance, ref status) in &self.stakers {
                log!(
                    trace,
                    "inserting genesis staker: {:?} => {:?} => {:?}",
                    stash,
                    balance,
                    status
                );
                assert!(
                    T::Balance::can_spend(&stash, balance),
                    "Stash does not have enough balance to bond."
                );

                frame_support::assert_ok!(<Pallet<T>>::validate_lock_for_staking(
                    T::Origin::from(Some(stash.clone()).into()),
                    T::Lookup::unlookup(controller.clone()),
                    balance
                ));

                frame_support::assert_ok!(<Pallet<T>>::bond(
                    stash.clone(),
                    controller.clone(),
                    balance
                ));

                frame_support::assert_ok!(match status {
                    crate::StakerStatus::Validator => <Pallet<T>>::apply_for_validator_role(
                        stash.clone(),
                        controller.clone(),
                        vec![],
                        balance,
                    ),
                    _ => Ok(().into()),
                });
            }
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// The stash account has been rewarded by this amount. \[utxo\]
        Rewarded(T::AccountId, Value),
        /// A new set of stakers was elected.
        StakersElected,
        /// An account has bonded this amount. \[stash, amount\]
        ///
        /// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
        /// it will not be emitted for staking rewards when they are added to stake.
        Bonded(T::AccountId, Value),
        /// An account has unbonded this amount. \[stash, amount\]
        Unbonded(T::AccountId, Value),
        /// An account has called `withdraw_unbonded` and removed unbonding chunks worth `Balance`
        /// from the unlocking queue. \[stash, amount\]
        Withdrawn(T::AccountId, Value),
        /// The election failed. No new era is planned.
        StakingElectionFailed,
        /// An account has stopped participating as validator.
        /// \[stash\]
        Chilled(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Not a controller account.
        NotController,
        /// Not a stash account.
        NotStash,
        /// Stash is already bonded.
        AlreadyBonded,
        /// Controller is already paired.
        AlreadyPaired,
        /// Targets cannot be empty.
        EmptyTargets,
        /// Duplicate index.
        DuplicateIndex,
        /// Can not bond with value less than minimum required.
        InsufficientBond,
        /// Not enough balance to perform the staking.
        InsufficientBalance,
        /// Can not schedule more unlock chunks.
        NoMoreChunks,
        /// Can not rebond without unlocking chunks.
        NoUnlockChunk,
        /// Attempting to target a stash that still has funds.
        FundedTarget,
        /// Invalid era to reward.
        InvalidEraToReward,
        /// Items are not sorted and unique.
        NotSortedAndUnique,
        /// Incorrect previous history depth input provided.
        IncorrectHistoryDepth,
        /// Internal state has become somehow corrupted and the operation cannot continue.
        BadState,
        /// There are too many validators in the system. Governance needs to adjust the staking
        /// settings to keep things safe for the runtime.
        TooManyValidators,
        /// Failed to decode the provided session key.
        /// Make sure to get it from the rpc call `author_rotateKeys`
        CannotDecodeSessionKey,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            // just return the weight of the on_finalize.
            T::DbWeight::get().reads(1)
        }

        fn on_finalize(_n: BlockNumberFor<T>) {
            // Set the start of the first era.
            if let Some(mut active_era) = Self::active_era() {
                if active_era.start.is_none() {
                    let now_as_millis_u64 = T::UnixTime::now().as_millis().saturated_into::<u64>();
                    active_era.start = Some(now_as_millis_u64);
                    // This write only ever happens once, we don't include it in the weight in
                    // general
                    ActiveEra::<T>::put(active_era);
                }
            }
            // `on_finalize` weight is tracked in `on_initialize`
        }

        fn integrity_test() {
            sp_std::if_std! {
                // sp_io::TestExternalities::new_empty().execute_with(||
                // 	assert!(
                // 		T::SlashDeferDuration::get() < T::BondingDuration::get() || T::BondingDuration::get() == 0,
                // 		"As per documentation, slash defer duration ({}) should be less than bonding duration ({}).",
                // 		T::SlashDeferDuration::get(),
                // 		T::BondingDuration::get(),
                // 	)
                // );
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::lock())]
        pub fn lock(
            origin: OriginFor<T>,
            controller: <T::Lookup as StaticLookup>::Source,
            session_keys: Vec<u8>,
            value: Value,
        ) -> DispatchResultWithPostInfo {
            Self::lock_for_staking(origin, controller, session_keys, value)
        }
    }
}
