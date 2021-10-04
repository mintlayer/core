
use crate::{Config, StakingCount, Error,Event, Pallet, TransactionOutput, Destination, LockedUtxos, Value, UtxoStore};
use sp_core::H256;
use frame_support::{dispatch::{DispatchResultWithPostInfo, Vec}, ensure};

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

    /// quitting the role of a validator.
    fn pause(controller_account:&AccountId) -> DispatchResultWithPostInfo;

    /// transfer balance from the locked state to the actual free balance.
    fn withdraw(controller_account: &AccountId) -> DispatchResultWithPostInfo;

}

/// performs the staking outside of the `pallet-utxo`.
pub(crate) fn stake<T: Config>(
    stash_account: &T::AccountId,
    controller_account: &T::AccountId,
    rotate_keys:&Vec<u8>)
    -> DispatchResultWithPostInfo {
    if <StakingCount<T>>::contains_key(controller_account) {
        Err(Error::<T>::StakingAlreadyExists)?
    }

    let mut rotate_keys = rotate_keys.to_vec();
    T::StakingHelper::stake(stash_account, controller_account, &mut rotate_keys)?;

    Ok(().into())
}

/// unlocking the staked funds outside of the `pallet-utxo`.
/// also means quitting/pausing from being a validator.
pub(crate) fn unlock<T: Config>(controller_pubkey: &H256) -> DispatchResultWithPostInfo {
    let controller_account= T::StakingHelper::get_account_id(controller_pubkey);
    T::StakingHelper::pause(&controller_account)?;

    Ok(().into())
}


/// Consolidates all unlocked utxos  into one, and moves it to `UtxoStore`.
/// Make SURE that `fn unlock(...)` has been called and the era for withdrawal has passed, before
/// performing a withdrawal.
/// # Arguments
/// * `output` - the output containing the `Destination::WithdrawStake`.
/// * `hash` - is the outpoint/key to save the new utxo
pub(crate) fn withdraw<T: Config>(output:&TransactionOutput<T::AccountId>,hash:H256) -> DispatchResultWithPostInfo {
    if let Destination::WithdrawStake { outpoints, pub_key } =  &output.destination {
        let controller_account = T::StakingHelper::get_account_id(pub_key);
        T::StakingHelper::withdraw(&controller_account)?;

        let mut sum:Value = 0;
        outpoints.iter().for_each(|hash| sum += <LockedUtxos<T>>::take(hash).unwrap().value);

        let new_tx = TransactionOutput {
            value: sum,
            header: 0,
            destination: Destination::Pubkey(pub_key.clone())
        };

        if !<UtxoStore<T>>::contains_key(hash) {
            log::info!("inserting to UtxoStore {:?} as key {:?}", new_tx, hash);
            <UtxoStore<T>>::insert(hash, Some(new_tx));
            <StakingCount<T>>::remove(controller_account);

            <Pallet<T>>::deposit_event(Event::<T>::StakeWithdrawn(sum,pub_key.clone()));
        }
    } else {
        log::error!("InvalidOperation; when performing a withdrawal, make sure the destination is `WithdrawStake`");
        Err(Error::<T>::InvalidOperation)?
    }

    Ok(().into())
}

/// Checks whether a given pub_key is a validator.
pub(crate) fn check_controller<T:Config>(controller_pubkey: &H256) -> Option<T::AccountId> {
    let controller_account = T::StakingHelper::get_account_id(controller_pubkey);
    if !<StakingCount<T>>::contains_key(&controller_account) {
        log::error!("No staking record found.");
        return None;
    }
    Some(controller_account.clone())
}


/// Checks a given hash outpoint/key is owned by the specified pub_key
/// # Arguments
/// * `hash` - is the outpoint/key of the locked utxo.
/// * `ctrl_pub_key` - the controller_pub_key, said to "own" the locked utxo
/// * `ctrl_account` - the account derivation of the controller_pub_key
pub(crate) fn is_owned_locked_utxo<T:Config>(hash:&H256, ctrl_pub_key: &H256, ctrl_account: &T::AccountId) -> Result<(), &'static str> {
    if let Some(utxo) = <LockedUtxos<T>>::get(hash) {
        match utxo.destination {
            Destination::Stake { stash_account:_, controller_account, rotate_keys: _ } => {
                ensure!(&controller_account == ctrl_account, "hash of stake not owned");

            }
            Destination::StakeExtra(controller_pub_key) => {
                ensure!(&controller_pub_key == ctrl_pub_key, "hash of extra stake not owned");
            }
            _ => {
                log::error!("For locked utxos, only with destinations `Stake` and `StakeExtra` are allowed.");
                Err("destination not applicable")?
            }
        }
    } else {
        Err("hash not found in lockedutxos.")?
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
pub(crate) fn validate_withdrawal<T:Config>(controller_pub_key: &H256, outpoints:&Vec<H256>) -> Result<(), &'static str> {
    let controller_account = T::StakingHelper::get_account_id(controller_pub_key);
    if <StakingCount<T>>::contains_key(controller_account.clone()) {
        if <StakingCount<T>>::get(&controller_account) == outpoints.len() as u64 {
            for hash in outpoints {
                is_owned_locked_utxo::<T>(hash,controller_pub_key,&controller_account)?;
            }
        } else {
            Err("unexpected number of outpoints.")?
        }
    } else {
        Err("controller_pub_key not a validator")?
    }
    Ok(())
}