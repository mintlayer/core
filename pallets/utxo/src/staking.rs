// Copyright (c) 2021 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://spdx.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Author(s): C. Yap

use crate::{Config, StakingCount, Error, Event, Pallet, TransactionOutput, Destination, LockedUtxos, Value, UtxoStore, convert_to_h256, RewardTotal};
use frame_support::{dispatch::{DispatchResultWithPostInfo, Vec}, ensure, traits::Get};
use sp_core::H256;
use sp_std::vec;
use sp_runtime::transaction_validity::{ValidTransaction,TransactionLongevity};
use sp_runtime::traits::{BlakeTwo256, Hash};

/// A helper trait to handle staking NOT found in pallet-utxo.
pub trait StakingHelper<AccountId>{

    /// to convert a public key into an AccountId
    fn get_account_id(pubkey: &H256) -> AccountId;

    /// start the staking.
    /// # Arguments
    /// * `stash_account` - A placeholder of the "supposed" validator. This is only to "satisfy"
    /// the `pallet-staking`'s needs to be able to stake.
    /// * `controller_account` - The ACTUAL validator. But this is NOT SO, in the `pallet-staking`.
    /// In `pallet-staking`, its job is like an "accountant" to the stash account.
    /// * `session_key` - to get up-to-date with validators, eras, sessions. see `pallet-session`.
    /// * `value` - the amount to stake/bond/stash
    fn lock_for_staking(stash_account:&AccountId, controller_account:&AccountId, session_key:&Vec<u8>, value:Value) -> DispatchResultWithPostInfo;

    /// stake more funds for the validator
    fn stake_extra(controller_account:&AccountId, value:Value) -> DispatchResultWithPostInfo;

    fn unlock_request_for_withdrawal(controller_account:&AccountId) -> DispatchResultWithPostInfo;

    /// transfer balance from the locked state to the actual free balance.
    fn withdraw(controller_account: &AccountId) -> DispatchResultWithPostInfo;

}

/// performs the staking OUTSIDE of the `pallet-utxo`. Calls the `fn stake(...)` function of
/// `StakingHelper` trait.
/// # Arguments
/// * `stash_pubkey` - is a `validator` in terms of `pallet-staking`; but in the utxo, this is nothing.
/// This is only to satisfy the `pallet-staking`. BUT this acts as a "bank", so this account MUST have
/// funds in its `pallet-balances` counterpart.
/// * `controller_pubkey` - is the `validator` in our utxo system.
/// * `session_key` - to get up-to-date with the validators, eras, sessions. see `pallet-session`.
/// * `value` - the amount to stake/bond/stash
pub(crate) fn lock_for_staking<T: Config>(
    stash_pubkey: &H256,
    controller_pubkey: &H256,
    session_key:&Vec<u8>,
    value:Value
)
    -> DispatchResultWithPostInfo {
    ensure!(!<StakingCount<T>>::contains_key(controller_pubkey),Error::<T>::ControllerAccountAlreadyRegistered);

    let stash_account = T::StakingHelper::get_account_id(stash_pubkey);
    let controller_account = T::StakingHelper::get_account_id(controller_pubkey);

    T::StakingHelper::lock_for_staking(&stash_account, &controller_account, session_key,value)
}

/// stake more values. This is only for existing validators.
pub(crate) fn stake_extra<T: Config>(controller_pubkey: &H256, value:Value) -> DispatchResultWithPostInfo {
    // Checks whether a given pubkey is a validator
    ensure!(<StakingCount<T>>::contains_key(controller_pubkey.clone()) , Error::<T>::ControllerAccountNotFound);

    let controller_account= T::StakingHelper::get_account_id(controller_pubkey);

    T::StakingHelper::stake_extra(&controller_account,value)
}

/// unlocking the staked funds outside of the `pallet-utxo`.
/// also means quitting/pausing from being a validator.
pub(crate) fn unlock_request_for_withdrawal<T: Config>(controller_pubkey: &H256) -> DispatchResultWithPostInfo {
    let controller_account= T::StakingHelper::get_account_id(controller_pubkey);
    T::StakingHelper::unlock_request_for_withdrawal(&controller_account)?;

    <Pallet<T>>::deposit_event(Event::<T>::StakeUnlocked(controller_pubkey.clone()));

    Ok(().into())
}

/// Consolidates all unlocked utxos  into one, and moves it to `UtxoStore`.
/// Make SURE that `fn unlock(...)` has been called and the era for withdrawal has passed, before
/// performing a withdrawal.
/// # Arguments
/// * `controller_pubkey` - the public key of the validator. In terms of pallet-staking, that's the
/// controller, NOT the stash.
/// * `outpoints` - a list of outpoints that were staked.
pub(crate) fn withdraw<T: Config>(controller_pubkey: H256, outpoints: Vec<H256>) -> DispatchResultWithPostInfo {
    validate_withdrawal::<T>(&controller_pubkey,&outpoints)?;

    let controller_account = T::StakingHelper::get_account_id(&controller_pubkey);
    T::StakingHelper::withdraw(&controller_account)?;

    let (_, mut total) = <StakingCount<T>>::get(controller_pubkey.clone()).ok_or("cannot find the public key inside the stakingcount.")?;

    let fee = T::StakeWithdrawalFee::get();
    total = total.checked_sub(fee).ok_or( "Total amount of Locked UTXOs is less than minimum?")?;

    let mut hashes = vec![];
    for hash in outpoints {
        <LockedUtxos<T>>::remove(hash);
        hashes.push(hash);
    }
    let hash = BlakeTwo256::hash_of(&hashes);

    let utxo = TransactionOutput::new_pubkey(total, controller_pubkey);
    <UtxoStore<T>>::insert(hash, Some(utxo));

    // TODO: currently moving fee back to the rewards.
    let reward_total = <RewardTotal<T>>::take();
    <RewardTotal<T>>::put(reward_total + fee);

    <Pallet<T>>::deposit_event(Event::<T>::StakeWithdrawn(total,controller_pubkey));
    Ok(().into())
}


pub(crate) fn is_owned_locked_utxo<T:Config>(utxo:&TransactionOutput<T::AccountId>, ctrl_pubkey: &H256) -> Result<(), &'static str> {
    match &utxo.destination {
        //TODO: change back to Public/H256 or something, after UI testing.
        Destination::LockForStaking { stash_account:_, controller_account, session_key:_ } => {
            let controller_pubkey = convert_to_h256::<T>(controller_account)?;
            ensure!(&controller_pubkey == ctrl_pubkey, "hash of stake not owned");
        }
        //TODO: change back to Public/H256 or something, after UI testing.
        Destination::StakeExtra(controller_account) => {
            let controller_pubkey = convert_to_h256::<T>(controller_account)?;
            ensure!(&controller_pubkey == ctrl_pubkey, "hash of extra stake not owned");
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
/// * `controller_pubkey` - An H256 public key of an account
/// * `outpoints` - List of keys of unlocked utxos said to be "owned" by the controller_pubkey
pub fn validate_withdrawal<T:Config>(controller_pubkey: &H256, outpoints:&Vec<H256>) -> Result<ValidTransaction, &'static str> {
    ensure!(<StakingCount<T>>::contains_key(controller_pubkey.clone()),Error::<T>::ControllerAccountNotFound);

    let (num_of_utxos, _) = <StakingCount<T>>::get(controller_pubkey.clone()).ok_or("cannot find the public key inside the stakingcount.")?;
    ensure!(num_of_utxos == outpoints.len() as u64, "please provide all staked outpoints.");

    let mut hashes = vec![];
    for hash in outpoints {
        ensure!(<LockedUtxos<T>>::contains_key(hash), Error::<T>::OutpointDoesNotExist);

        let utxo =  <LockedUtxos<T>>::get(hash).ok_or(Error::<T>::OutpointDoesNotExist)?;
        is_owned_locked_utxo::<T>(&utxo,controller_pubkey)?;
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