use crate::{Config, Value, Bonded, Error, Ledger, Pallet, MaxValidatorsCount, CounterForValidators, SettingSessionKey};
use frame_system::pallet_prelude::OriginFor;
use frame_support::ensure;
use frame_support::sp_runtime::traits::StaticLookup;
use frame_support::dispatch::{DispatchResultWithPostInfo, DispatchResult};
use frame_system::ensure_signed;
use sp_std::vec::Vec;
use sp_runtime::DispatchError;


/// A helper trait to expose the balance of the account
pub trait Balance<AccountId> {
    fn staking_fee() -> Value;

    fn minimum_stake_balance() -> Value;

    fn can_spend(account:&AccountId, value:Value) -> bool;

    fn lock_for_staking(stash: AccountId, controller: AccountId, session_keys: Vec<u8>, value: Value) -> DispatchResultWithPostInfo;
}

impl <T:Config> Pallet<T> {
    /// Checks if origin account is not yet a stash account
    /// Checks if controller account has not yet been paired to a stash account
    /// Checks if the minimum stake balance is reached
    /// Checks if the session key can be decoded
    /// returns a tuple of (Stash,Controller)
    pub fn validate_lock_for_staking(
        origin: OriginFor<T>,
        controller: <T::Lookup as StaticLookup>::Source,
        session_keys:&Vec<u8>,
        value: u128
    ) -> Result<(T::AccountId,T::AccountId), DispatchError> {
        let stash = ensure_signed(origin)?;
        let controller = T::Lookup::lookup(controller)?;

        ensure!(
            !<Bonded<T>>::contains_key(&stash),
            Error::<T>::AlreadyBonded
        );

        ensure!(
            T::Balance::can_spend(&stash, value + T::Balance::staking_fee()),
            Error::<T>::InsufficientBalance
        );

        ensure!(
            value >= T::Balance::minimum_stake_balance(),
            Error::<T>::InsufficientBond
        );

        ensure!(
            !<Ledger<T>>::contains_key(&controller),
            Error::<T>::AlreadyPaired
        );

        // check only if we've set a limit to the maximum number of validators
        if let Some(max_validators) = <MaxValidatorsCount<T>>::get() {
            // If this error is reached, we need to adjust the `MinValidatorBond` and start
            // calling `chill_other`. Until then, we explicitly block new validators to protect
            // the runtime.
            ensure!(
                <CounterForValidators<T>>::get() < max_validators,
                Error::<T>::TooManyValidators
            );
        }

        ensure!(
            T::SettingSessionKey::can_decode_session_keys(session_keys),
            Error::<T>::CannotDecodeSessionKey
        );

        Ok((stash,controller))
    }


    /// Take the origin account as a stash and lock up `value` of its balance. `controller` will
   /// be the account that controls it.
   ///
   /// `value` must be more than the `minimum_balance`.
   /// emits `Bonded`
    pub(crate) fn bond(
        stash:T::AccountId,
        controller:T::AccountId,
        value:Value
    ) -> DispatchResult {
        frame_system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;

        // You're auto-bonded forever, here.
        <Bonded<T>>::insert(&stash, &controller);
        Self::add_ledger(stash,controller,value);

      Ok(())
    }

    pub(crate) fn apply_for_validator_role(
        stash: T::AccountId,
        controller: T::AccountId,
        session_keys:Vec<u8>,
        value: Value
    ) -> DispatchResultWithPostInfo {
        Self::do_add_validator(&stash,&controller);
        T::Balance::lock_for_staking(stash,controller, session_keys,value)
    }

    /// Calls the bond function, with the origin account as a stash.
    /// The session key should be from the rpc call `author_rotateKeys` or similar.
    ///
    /// The dispatch origin for this call must be _Signed_ by the stash account.
    ///
    pub fn lock_for_staking(
        origin: OriginFor<T>,
        controller: <T::Lookup as StaticLookup>::Source,
        session_keys:Vec<u8>,
        value: Value
    ) -> DispatchResultWithPostInfo {
        let (stash, controller) =
            Self::validate_lock_for_staking(origin,controller,&session_keys, value)?;

        Self::bond(stash.clone(),controller.clone(),value)?;

        // set session key here
        T::SettingSessionKey::set_session_keys(controller.clone(),&session_keys)?;

        Self::apply_for_validator_role(stash,controller,session_keys,value)
    }

    pub fn lock_extra_for_staking(
        origin: OriginFor<T>,
        max_additional: Value
    ) -> DispatchResultWithPostInfo {
        let stash = ensure_signed(origin)?;
        let controller = Self::bonded(&stash).ok_or(Error::<T>::NotStash)?;
        let mut ledger = Self::ledger(&controller).ok_or(Error::<T>::NotController)?;

        Self::update_ledger(stash,controller,max_additional,&mut ledger);

        Ok(().into())
    }


}

