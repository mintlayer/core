
use crate::{
    Config, StakingCount, Error, Event, Pallet, TransactionOutput, Destination, LockedUtxos,
    Value, UtxoStore, MLT_UNIT
};
use frame_support::{dispatch::{DispatchResultWithPostInfo, Vec}, ensure};
use sp_core::H256;
use sp_std::vec;
use sp_runtime::transaction_validity::{ValidTransaction,TransactionLongevity};
use sp_runtime::traits::{BlakeTwo256, Hash};

/// A helper trait to handle staking NOT found in pallet-utxo.
pub trait StakingHelper<AccountId>{

    fn get_account_id(pub_key: &H256) -> AccountId;

    /// start the staking.
    /// # Arguments
    /// * `stash_account` - A placeholder of the "supposed" validator. This is only to "satisfy"
    /// the `pallet-staking`'s needs to be able to stake.
    /// * `controller_account` - The ACTUAL validator. But this is NOT SO, in the `pallet-staking`.
    /// In `pallet-staking`, its job is like an "accountant" to the stash account.
    /// * `rotate_keys` - or also called the session keys, to get up-to-date
    /// with validators, eras, sessions. see `pallet-session`.
    fn stake(stash_account:&AccountId, controller_account:&AccountId, rotate_keys:&mut Vec<u8>) -> DispatchResultWithPostInfo;

    /// stake more funds for the validator
    fn stake_extra(controller_account:&AccountId, value:Value) -> DispatchResultWithPostInfo;

    /// quitting the role of a validator.
    fn pause(controller_account:&AccountId) -> DispatchResultWithPostInfo;

    /// transfer balance from the locked state to the actual free balance.
    fn withdraw(controller_account: &AccountId) -> DispatchResultWithPostInfo;

}

/// performs the staking outside of the `pallet-utxo`.
pub(crate) fn stake<T: Config>(
    stash_pubkey: &H256,
    controller_pubkey: &H256,
    session_key:&Vec<u8>)
    -> DispatchResultWithPostInfo {
    ensure!(!<StakingCount<T>>::contains_key(controller_pubkey),Error::<T>::StakingAlreadyExists);

    let mut session_key = session_key.to_vec();

    let stash_account = T::StakingHelper::get_account_id(stash_pubkey);
    let controller_account = T::StakingHelper::get_account_id(controller_pubkey);

    T::StakingHelper::stake(&stash_account, &controller_account, &mut session_key)?;

    Ok(().into())
}

/// stake more values. This is only for existing validators.
pub(crate) fn stake_extra<T: Config>(controller_pub_key: &H256, value:Value) -> DispatchResultWithPostInfo {
    // Checks whether a given pub_key is a validator
    ensure!(<StakingCount<T>>::contains_key(controller_pub_key.clone()) , Error::<T>::NoStakingRecordFound);

    let controller_account= T::StakingHelper::get_account_id(controller_pub_key);

    let non_mlt_value = value.checked_div(MLT_UNIT).unwrap();
    T::StakingHelper::stake_extra(&controller_account,non_mlt_value)?;

    Ok(().into())
}

/// unlocking the staked funds outside of the `pallet-utxo`.
/// also means quitting/pausing from being a validator.
pub(crate) fn unlock<T: Config>(controller_pubkey: &H256) -> DispatchResultWithPostInfo {
    let controller_account= T::StakingHelper::get_account_id(controller_pubkey);
    T::StakingHelper::pause(&controller_account)?;

    <Pallet<T>>::deposit_event(Event::<T>::StakeUnlocked(controller_pubkey.clone()));

    Ok(().into())
}

/// Consolidates all unlocked utxos  into one, and moves it to `UtxoStore`.
/// Make SURE that `fn unlock(...)` has been called and the era for withdrawal has passed, before
/// performing a withdrawal.
/// # Arguments
/// * `controller_pub_key` - the public key of the validator. In terms of pallet-staking, that's the
/// controller, NOT the stash.
/// * `outpoints` - a list of outpoints that were staked.
pub(crate) fn withdraw<T: Config>(controller_pub_key: H256, outpoints: Vec<H256>) -> DispatchResultWithPostInfo {
    let (_, mut total) = <StakingCount<T>>::get(controller_pub_key.clone());
    //  1 MLT as fee.
    total -= 1 * MLT_UNIT;
    // TODO: where do i put this fee? back to MLTCoinsAvailable?

    let mut hashes = vec![];
    for hash in outpoints {
        <LockedUtxos<T>>::remove(hash);
        hashes.push(hash);
    }
    let hash = BlakeTwo256::hash_of(&hashes);

    let utxo = TransactionOutput {
        value: total,
        header: 0,
        destination: Destination::Pubkey(controller_pub_key.clone())
    };

    <UtxoStore<T>>::insert(hash, Some(utxo));

    <Pallet<T>>::deposit_event(Event::<T>::StakeWithdrawn(total,controller_pub_key));
    Ok(().into())
}


pub(crate) fn is_owned_locked_utxo<T:Config>(utxo:&TransactionOutput<T::AccountId>, ctrl_pub_key: &H256) -> Result<(), &'static str> {
    match &utxo.destination {
        Destination::Stake { stash_pubkey:_, controller_pubkey, session_key:_ } => {
            ensure!(controller_pubkey == ctrl_pub_key, "hash of stake not owned");
        }
        Destination::StakeExtra(controller_pub_key) => {
            ensure!(controller_pub_key == ctrl_pub_key, "hash of extra stake not owned");
        }
        _ => {
            log::error!("For locked utxos, only with destinations `Stake` and `StakeExtra` are allowed.");
            Err("destination not applicable")?
        }
    }
    Ok(())
}

/// It includes:
/// 1. Check if the pub key is a controller.
/// 2. Checking the number of outpoints owned by the given pub key
/// 3. Checking each outpoints if they are indeed owned by the pub key
/// Returns a Result with an empty Ok, or an Err in string.
/// # Arguments
/// * `controller_pub_key` - An H256 public key of an account
/// * `outpoints` - List of keys of unlocked utxos said to be "owned" by the controller_pub_key
pub fn validate_withdrawal<T:Config>(controller_pub_key: &H256, outpoints:&Vec<H256>) -> Result<ValidTransaction, &'static str> {
    ensure!(<StakingCount<T>>::contains_key(controller_pub_key.clone()),Error::<T>::NoStakingRecordFound);

    let (num_of_utxos, _) = <StakingCount<T>>::get(controller_pub_key.clone());
    ensure!(num_of_utxos == outpoints.len() as u64, "please provide all staked outpoints.");

    let mut hashes = vec![];
    for hash in outpoints {
        ensure!(<LockedUtxos<T>>::contains_key(hash), Error::<T>::OutpointDoesNotExist);

        let utxo =  <LockedUtxos<T>>::get(hash).unwrap();
        is_owned_locked_utxo::<T>(&utxo,controller_pub_key)?;
        hashes.push(hash.clone());
    }

    let new_hash = BlakeTwo256::hash_of(&hashes).as_fixed_bytes().to_vec();

    Ok(ValidTransaction {
        priority: 1,
        requires: vec![],
        provides: vec![new_hash],
        longevity: TransactionLongevity::MAX,
        propagate: true
    })
}