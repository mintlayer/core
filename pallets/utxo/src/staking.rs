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

use crate::{Config, StakingCount, Error, Event, Pallet, TransactionOutput, Destination, LockedUtxos, Value, UtxoStore, MLT_UNIT, MLTCoinsAvailable};
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
    fn stake(stash_account:&AccountId, controller_account:&AccountId, session_key:&mut Vec<u8>, value:Value) -> DispatchResultWithPostInfo;

    /// stake more funds for the validator
    fn stake_extra(controller_account:&AccountId, value:Value) -> DispatchResultWithPostInfo;

    /// quitting the role of a validator.
    fn pause(controller_account:&AccountId) -> DispatchResultWithPostInfo;

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
pub(crate) fn stake<T: Config>(
    stash_pubkey: &H256,
    controller_pubkey: &H256,
    session_key:&Vec<u8>,
    value:Value
)
    -> DispatchResultWithPostInfo {
    ensure!(!<StakingCount<T>>::contains_key(controller_pubkey),Error::<T>::StakingAlreadyExists);

    let mut session_key = session_key.to_vec();

    let stash_account = T::StakingHelper::get_account_id(stash_pubkey);
    let controller_account = T::StakingHelper::get_account_id(controller_pubkey);

    let non_mlt_value = value.checked_div(MLT_UNIT).ok_or("could not convert to a non mlt value.")?;
    T::StakingHelper::stake(&stash_account, &controller_account, &mut session_key,non_mlt_value)
}

/// stake more values. This is only for existing validators.
pub(crate) fn stake_extra<T: Config>(controller_pubkey: &H256, value:Value) -> DispatchResultWithPostInfo {
    // Checks whether a given pubkey is a validator
    ensure!(<StakingCount<T>>::contains_key(controller_pubkey.clone()) , Error::<T>::NoStakingRecordFound);

    let controller_account= T::StakingHelper::get_account_id(controller_pubkey);

    let non_mlt_value = value.checked_div(MLT_UNIT).ok_or("could not convert to a non mlt value.")?;
    T::StakingHelper::stake_extra(&controller_account,non_mlt_value)
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

    let utxo = TransactionOutput {
        value: total,
        header: 0,
        destination: Destination::Pubkey(controller_pubkey.clone())
    };

    <UtxoStore<T>>::insert(hash, Some(utxo));

    // move fee back to the rewards
    let coins_available = <MLTCoinsAvailable<T>>::take();
    <MLTCoinsAvailable<T>>::put(coins_available + fee);

    <Pallet<T>>::deposit_event(Event::<T>::StakeWithdrawn(total,controller_pubkey));
    Ok(().into())
}


pub(crate) fn is_owned_locked_utxo<T:Config>(utxo:&TransactionOutput<T::AccountId>, ctrl_pubkey: &H256) -> Result<(), &'static str> {
    match &utxo.destination {
        Destination::Stake { stash_pubkey:_, controller_pubkey, session_key:_ } => {
            ensure!(controller_pubkey == ctrl_pubkey, "hash of stake not owned");
        }
        Destination::StakeExtra(controller_pubkey) => {
            ensure!(controller_pubkey == ctrl_pubkey, "hash of extra stake not owned");
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
    ensure!(<StakingCount<T>>::contains_key(controller_pubkey.clone()),Error::<T>::NoStakingRecordFound);

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

/// This is to make life easier, but the signing thing doesn't work yet,
/// so it's still a TODO.
pub mod calls {
    use super::*;
    use sp_core::{sr25519::Public as SR25519Public, testing::SR25519};
    use frame_support::sp_runtime::app_crypto::RuntimePublic;
    use crate::{Transaction, TransactionInput, spend};
    use codec::Encode;
    use core::convert::TryInto;

    pub fn stake<T: Config>(
        controller_account:T::AccountId,
        stash_account:T::AccountId,
        session_key:Vec<u8>,
        outpoint: H256,
        stake_value:Value
    ) -> DispatchResultWithPostInfo {

        let pubkey_raw: [u8; 32] = match controller_account.encode().try_into() {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to get caller's public key: {:?}", e);
                Err("Failed to get caller's public key")?
            }
        };
        let controller_pubkey = H256::from(pubkey_raw);
        let controller_public = SR25519Public::from_h256(controller_pubkey);

        let pubkey_raw: [u8; 32] = match stash_account.encode().try_into() {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to get stash account's public key: {:?}", e);
                Err("Failed to get stash account's public key")?
            }
        };
        let stash_pubkey = H256::from(pubkey_raw);

        let utxo =  <UtxoStore<T>>::get(outpoint).ok_or(
            Error::<T>::OutpointDoesNotExist
        )?;

        ensure!(utxo.value > stake_value + MLT_UNIT, Error::<T>::BalanceLow);

        let mut tx = Transaction {
            inputs: vec![TransactionInput::new_empty(outpoint)],
            outputs: vec![
                TransactionOutput::new_stake(stake_value,stash_pubkey,controller_pubkey,session_key),
                TransactionOutput::new_pubkey(utxo.value - (stake_value + MLT_UNIT), controller_pubkey)
            ]
        };

        let sig = controller_public.sign(SR25519, &tx.encode()).ok_or(Error::<T>::FailedSigningTransaction)?;
        tx.inputs[0].witness = sig.0.to_vec();

        spend::<T>(&controller_account,&tx)
    }

    pub fn stake_extra<T: Config>(
        controller_account:T::AccountId,
        outpoint: H256,
        stake_value:Value
    ) -> DispatchResultWithPostInfo {
        let pubkey_raw: [u8; 32] = match controller_account.encode().try_into() {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to get caller's public key: {:?}", e);
                Err("Failed to get caller's public key")?
            }
        };
        let controller_pubkey = H256::from(pubkey_raw);
        let controller_public = SR25519Public::from_h256(controller_pubkey);

        let utxo =  <UtxoStore<T>>::get(outpoint).ok_or(
            Error::<T>::OutpointDoesNotExist
        )?;

        ensure!(utxo.value > stake_value + MLT_UNIT, Error::<T>::BalanceLow);

        let mut tx = Transaction {
            inputs: vec![TransactionInput::new_empty(outpoint)],
            outputs: vec![
                TransactionOutput::new_stake_extra(stake_value,controller_pubkey),
                TransactionOutput::new_pubkey(utxo.value - (stake_value + MLT_UNIT), controller_pubkey)
            ]
        };

        let sig = controller_public.sign(SR25519, &tx.encode()).ok_or(Error::<T>::FailedSigningTransaction)?;
        tx.inputs[0].witness = sig.0.to_vec();

        spend::<T>(&controller_account,&tx)
    }
}