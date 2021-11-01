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

use crate::{
    convert_to_h256, Config, Destination, Error, Event, LockedUtxos, Pallet, RewardTotal,
    StakingCount, TransactionOutput, UtxoStore, Value,
};
use frame_support::{
    dispatch::{DispatchResultWithPostInfo, Vec},
    ensure, fail,
    traits::Get,
};
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Hash};
use sp_runtime::transaction_validity::{TransactionLongevity, ValidTransaction};
use sp_std::vec;

pub use validation::*;

/// A helper trait to handle staking NOT found in pallet-utxo.
pub trait StakingHelper<AccountId> {
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
    fn lock_for_staking(
        stash_account: &AccountId,
        controller_account: &AccountId,
        session_key: &Vec<u8>,
        value: Value,
    ) -> DispatchResultWithPostInfo;

    /// stake more funds for the validator
    fn lock_extra_for_staking(
        controller_account: &AccountId,
        value: Value,
    ) -> DispatchResultWithPostInfo;

    fn unlock_request_for_withdrawal(controller_account: &AccountId) -> DispatchResultWithPostInfo;

    /// transfer balance from the locked state to the actual free balance.
    fn withdraw(controller_account: &AccountId) -> DispatchResultWithPostInfo;
}

/// unlocking the staked funds outside of the `pallet-utxo`.
/// also means you don't want to be a validator anymore.
pub(crate) fn unlock_request_for_withdrawal<T: Config>(
    controller_account: T::AccountId,
) -> DispatchResultWithPostInfo {
    let res = T::StakingHelper::unlock_request_for_withdrawal(&controller_account)?;
    let controller_pubkey = convert_to_h256::<T>(&controller_account)?;
    <Pallet<T>>::deposit_event(Event::<T>::StakeUnlocked(controller_pubkey));
    Ok(res)
}

/// Consolidates all unlocked utxos  into one, and moves it to `UtxoStore`.
/// Make SURE that `fn unlock(...)` has been called and the era for withdrawal has passed, before
/// performing a withdrawal.
/// # Arguments
/// * `controller_pubkey` - the public key of the validator. In terms of pallet-staking, that's the
/// controller, NOT the stash.
/// * `outpoints` - a list of outpoints that were staked.
pub(crate) fn withdraw<T: Config>(
    controller_account: T::AccountId,
    outpoints: Vec<H256>,
) -> DispatchResultWithPostInfo {
    let controller_pubkey = convert_to_h256::<T>(&controller_account)?;
    validate_withdrawal::<T>(&controller_pubkey, &outpoints)?;

    let res = T::StakingHelper::withdraw(&controller_account)?;

    let (_, mut total) = <StakingCount<T>>::take(controller_pubkey)
        .ok_or("cannot find the public key inside the stakingcount.")?;

    let fee = T::StakeWithdrawalFee::get();
    total = total
        .checked_sub(fee)
        .ok_or("Total amount of Locked UTXOs is less than minimum?")?;

    outpoints.iter().for_each(|hash| <LockedUtxos<T>>::remove(hash));

    let hash = BlakeTwo256::hash_of(&outpoints);
    let utxo = TransactionOutput::new_pubkey(total, controller_pubkey);
    <UtxoStore<T>>::insert(hash, utxo);

    let reward_total = <RewardTotal<T>>::take();
    <RewardTotal<T>>::put(reward_total + fee);

    <Pallet<T>>::deposit_event(Event::<T>::StakeWithdrawn(total, controller_pubkey));
    Ok(res)
}

/// Calls the outside staking logic to lock some funds
/// Adds the transaction output to the `LockedUtxos` storage and `StakingCount` storage.
pub(crate) fn lock_for_staking<T: Config>(
    hash_key: H256,
    output: &TransactionOutput<T::AccountId>,
) -> DispatchResultWithPostInfo {
    //TODO: change back to Public/H256 or something, after UI testing.
    if let Destination::LockForStaking {
        stash_account,
        controller_account,
        session_key,
    } = &output.destination
    {
        T::StakingHelper::lock_for_staking(
            stash_account,
            controller_account,
            session_key,
            output.value,
        )?;
        let controller_pubkey = convert_to_h256::<T>(controller_account)?;
        return utils::add_to_locked_utxos::<T>(hash_key, output, controller_pubkey);
    }
    fail!(Error::<T>::InvalidOperation)
}

/// For existing stakers who wants to add more utxos to lock.
/// Also calls the outside staking logic to lock these extra funds.
pub(crate) fn locking_extra_utxos<T: Config>(
    hash_key: H256,
    output: &TransactionOutput<T::AccountId>,
) -> DispatchResultWithPostInfo {
    //TODO: change back to Public/H256 or something, after UI testing.
    if let Destination::LockExtraForStaking(controller_account) = &output.destination {
        T::StakingHelper::lock_extra_for_staking(controller_account, output.value)?;
        let controller_pubkey = convert_to_h256::<T>(controller_account)?;
        // Checks whether a given pubkey is a validator
        ensure!(
            <StakingCount<T>>::contains_key(controller_pubkey),
            Error::<T>::ControllerAccountNotFound
        );
        return utils::add_to_locked_utxos::<T>(hash_key, output, controller_pubkey);
    }
    fail!(Error::<T>::InvalidOperation)
}

mod utils {
    use super::*;
    use sp_runtime::DispatchError;

    pub fn is_owned_locked_utxo<T: Config>(
        utxo: &TransactionOutput<T::AccountId>,
        ctrl_pubkey: &H256,
    ) -> Result<(), &'static str> {
        match &utxo.destination {
            //TODO: change back to Public/H256 or something, after UI testing.
            Destination::LockForStaking {
                stash_account: _,
                controller_account,
                session_key: _,
            } => {
                let controller_pubkey = convert_to_h256::<T>(controller_account)?;
                ensure!(&controller_pubkey == ctrl_pubkey, "hash of stake not owned");
            }
            //TODO: change back to Public/H256 or something, after UI testing.
            Destination::LockExtraForStaking(controller_account) => {
                let controller_pubkey = convert_to_h256::<T>(controller_account)?;
                ensure!(
                    &controller_pubkey == ctrl_pubkey,
                    "hash of extra stake not owned"
                );
            }
            _ => {
                log::error!("For locked utxos, only with destinations `Stake` and `StakeExtra` are allowed.");
                Err("destination not applicable")?
            }
        }
        Ok(())
    }

    /// adds to the `LockedUtxo` storage
    /// add to the `StakingCount` storage
    pub fn add_to_locked_utxos<T: Config>(
        hash_key: H256,
        output: &TransactionOutput<T::AccountId>,
        controller_pubkey: H256,
    ) -> DispatchResultWithPostInfo {
        log::debug!(
            "Locking utxo({:?}) of controller {:?}",
            hash_key,
            controller_pubkey
        );
        let (num_of_utxos, total) = <StakingCount<T>>::get(&controller_pubkey).unwrap_or((0, 0));
        <StakingCount<T>>::insert(
            controller_pubkey,
            (
                num_of_utxos.checked_add(1).ok_or(DispatchError::Other(
                    "exceeded limit of total number of utxos locked",
                ))?,
                total
                    .checked_add(output.value)
                    .ok_or(DispatchError::Other("exceeded limit of total utxos locked"))?,
            ),
        );

        log::debug!(
            "inserting to LockedUtxos {:?} as key {:?}",
            output,
            hash_key
        );
        <LockedUtxos<T>>::insert(hash_key, output);

        Ok(().into())
    }
}

pub mod validation {
    use super::*;

    /// Checks whether a transaction is valid to do `lock_for_staking`.
    pub(crate) fn validate_lock_for_staking_requirements<T: Config>(
        hash_key: H256,
        output_value: Value,
        controller_account: &T::AccountId,
    ) -> Result<(), &'static str> {
        let controller_pubkey = convert_to_h256::<T>(controller_account)?;
        // Checks whether a given pubkey is already a validator
        ensure!(
            !<StakingCount<T>>::contains_key(controller_pubkey),
            Error::<T>::ControllerAccountAlreadyRegistered
        );

        ensure!(
            output_value >= T::MinimumStake::get(),
            "output value must be equal or more than the set minimum stake"
        );
        ensure!(
            !<LockedUtxos<T>>::contains_key(hash_key),
            "output already exists in the LockedUtxos storage"
        );
        Ok(())
    }

    /// Checks whether a transaction is valid to do extra locking of utxos for staking
    pub(crate) fn validate_lock_extra_for_staking_requirements<T: Config>(
        hash_key: H256,
        output_value: Value,
    ) -> Result<(), &'static str> {
        ensure!(output_value > 0, "output value must be nonzero");
        ensure!(
            !<LockedUtxos<T>>::contains_key(hash_key),
            Error::<T>::OutpointAlreadyExist
        );
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
    pub fn validate_withdrawal<T: Config>(
        controller_pubkey: &H256,
        outpoints: &Vec<H256>,
    ) -> Result<ValidTransaction, &'static str> {
        ensure!(
            <StakingCount<T>>::contains_key(controller_pubkey),
            Error::<T>::ControllerAccountNotFound
        );

        let (num_of_utxos, _) = <StakingCount<T>>::get(controller_pubkey)
            .ok_or("cannot find the public key inside the stakingcount.")?;
        ensure!(
            num_of_utxos == outpoints.len() as u64,
            "please provide all staked outpoints."
        );

        for hash in outpoints {
            ensure!(
                <LockedUtxos<T>>::contains_key(hash),
                Error::<T>::OutpointDoesNotExist
            );

            let utxo = <LockedUtxos<T>>::get(hash).ok_or(Error::<T>::OutpointDoesNotExist)?;
            utils::is_owned_locked_utxo::<T>(&utxo, controller_pubkey)?;
        }

        let new_hash = BlakeTwo256::hash_of(&outpoints).as_fixed_bytes().to_vec();

        Ok(ValidTransaction {
            priority: 1,
            requires: vec![],
            provides: vec![new_hash],
            longevity: TransactionLongevity::MAX,
            propagate: true,
        })
    }
}
