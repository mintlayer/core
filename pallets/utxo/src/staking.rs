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
    fn get_controller_account(stash_account: &AccountId) -> Result<AccountId, &'static str>;

    fn is_controller_account_exist(controller_account: &AccountId) -> bool;

    fn can_decode_session_key(session_key: &Vec<u8>) -> bool;

    fn are_funds_locked(controller_account: &AccountId) -> bool;

    fn check_accounts_matched(controller_account: &AccountId, stash_account: &AccountId) -> bool;

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
        stash_account: &AccountId,
        controller_account: &AccountId,
        value: Value,
    ) -> DispatchResultWithPostInfo;

    fn unlock_request_for_withdrawal(stash_account: &AccountId) -> DispatchResultWithPostInfo;

    /// transfer balance from the locked state to the actual free balance.
    fn withdraw(stash_account: &AccountId) -> DispatchResultWithPostInfo;
}

/// Calls the outside staking logic to lock some funds
/// Adds the transaction output to the `LockedUtxos` storage and `StakingCount` storage.
pub(crate) fn lock_for_staking<T: Config>(
    hash_key: H256,
    output: &TransactionOutput<T::AccountId>,
) -> DispatchResultWithPostInfo {
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
        return utils::add_to_locked_utxos::<T>(hash_key, output, stash_account);
    }
    fail!(Error::<T>::InvalidOperation)
}

/// For existing stakers who wants to add more utxos to lock.
/// Also calls the outside staking logic to lock these extra funds.
pub(crate) fn lock_extra_for_staking<T: Config>(
    hash_key: H256,
    output: &TransactionOutput<T::AccountId>,
) -> DispatchResultWithPostInfo {
    if let Destination::LockExtraForStaking {
        stash_account,
        controller_account,
    } = &output.destination
    {
        T::StakingHelper::lock_extra_for_staking(stash_account, controller_account, output.value)?;
        return utils::add_to_locked_utxos::<T>(hash_key, output, stash_account);
    }
    fail!(Error::<T>::InvalidOperation)
}

/// unlocking the staked funds outside of the `pallet-utxo`.
/// also means you don't want to be a validator anymore.
pub(crate) fn unlock_request_for_withdrawal<T: Config>(
    stash_account: T::AccountId,
) -> DispatchResultWithPostInfo {
    validate_unlock_request_for_withdrawal::<T>(&stash_account)?;

    let res = T::StakingHelper::unlock_request_for_withdrawal(&stash_account)?;
    <Pallet<T>>::deposit_event(Event::<T>::StakeUnlocked(stash_account));
    Ok(res)
}

/// Consolidates all unlocked utxos  into one, and moves it to `UtxoStore`.
/// Make SURE that `fn unlock(...)` has been called and the era for withdrawal has passed, before
/// performing a withdrawal.
pub(crate) fn withdraw<T: Config>(
    stash_account: T::AccountId,
    outpoints: Vec<H256>,
) -> DispatchResultWithPostInfo {
    validate_withdrawal::<T>(&stash_account, &outpoints)?;

    let res = T::StakingHelper::withdraw(&stash_account)?;

    let stash_pubkey = convert_to_h256::<T>(&stash_account)?;
    let (_, mut total) =
        <StakingCount<T>>::take(stash_account.clone()).ok_or(Error::<T>::StashAccountNotFound)?;

    let fee = T::StakeWithdrawalFee::get();
    total = total
        .checked_sub(fee)
        .ok_or("Total amount of Locked UTXOs is less than minimum?")?;

    outpoints.iter().for_each(|hash| <LockedUtxos<T>>::remove(hash));

    let hash = BlakeTwo256::hash_of(&outpoints);
    // move locked utxo back to UtxoStore
    let utxo = TransactionOutput::new_pubkey(total, stash_pubkey);
    <UtxoStore<T>>::insert(hash, utxo);

    // insert the fee into the reward total
    let reward_total = <RewardTotal<T>>::take();
    <RewardTotal<T>>::put(reward_total + fee);

    <Pallet<T>>::deposit_event(Event::<T>::StakeWithdrawn(total, stash_account));
    Ok(res)
}

pub mod validation {
    use super::*;
    use crate::{OutputHeaderHelper, TokenType, TransactionOutputFor};

    /// to validate `LockForStaking` and `LockExtraForStaking`
    pub fn validate_staking_ops<T: Config>(
        tx: &TransactionOutputFor<T>,
        hash_key: H256,
    ) -> DispatchResultWithPostInfo {
        ensure!(
            !<LockedUtxos<T>>::contains_key(hash_key),
            "output already exists in the LockedUtxos storage"
        );

        ensure!(
            matches!(
                tx.header.as_tx_output_header().token_type(),
                Some(TokenType::MLT)
            ),
            "only MLT tokens are supported for staking"
        );

        // call individual validation functions
        match &tx.destination {
            Destination::LockForStaking {
                stash_account,
                controller_account,
                session_key,
            } => {
                ensure!(
                    tx.value >= T::MinimumStake::get(),
                    "output value must be equal or more than the minimum stake"
                );
                validate_lock_for_staking_requirements::<T>(
                    stash_account,
                    controller_account,
                    session_key,
                )
            }
            Destination::LockExtraForStaking {
                stash_account,
                controller_account,
            } => {
                ensure!(tx.value > 0, "output value must be nonzero");
                validate_lock_extra_for_staking_requirements::<T>(stash_account, controller_account)
            }
            _non_staking_destinations => {
                fail!(Error::<T>::InvalidOperation)
            }
        }
    }

    /// Checks whether a transaction is valid to do `lock_for_staking`.
    fn validate_lock_for_staking_requirements<T: Config>(
        stash_account: &T::AccountId,
        controller_account: &T::AccountId,
        session_key: &Vec<u8>,
    ) -> DispatchResultWithPostInfo {
        // ---------- check for clearance INSIDE the utxo system. ----------
        ensure!(
            !<StakingCount<T>>::contains_key(stash_account.clone()),
            Error::<T>::StashAccountAlreadyRegistered
        );

        // ---------- check for clearance OUTSIDE the utxo system. ----------
        ensure!(
            T::StakingHelper::can_decode_session_key(session_key),
            "please input a valid session key."
        );

        ensure!(
            !T::StakingHelper::is_controller_account_exist(stash_account),
            "specified stash account is a controller account"
        );

        ensure!(
            !T::StakingHelper::is_controller_account_exist(controller_account),
            "specified controller account is already used."
        );

        Ok(().into())
    }

    /// Checks whether a transaction is valid to do extra locking of utxos for staking
    fn validate_lock_extra_for_staking_requirements<T: Config>(
        stash_account: &T::AccountId,
        controller_account: &T::AccountId,
    ) -> DispatchResultWithPostInfo {
        ensure!(
            <StakingCount<T>>::contains_key(stash_account),
            Error::<T>::StashAccountNotFound
        );

        // if the funds are already unlocked, it means you should withdraw them first,
        // then perform a `LockForStaking`.
        ensure!(
            T::StakingHelper::are_funds_locked(controller_account),
            "Cannot perform locking of stake when pending unlocked funds are not withdrawn."
        );

        // the pair should match. 1 stash account has 1 controller account, and vice versa.
        ensure!(
            T::StakingHelper::check_accounts_matched(controller_account, stash_account),
            "the provided account pairs do not match."
        );

        Ok(().into())
    }

    /// Checks whether unlock request is allowed.
    pub(crate) fn validate_unlock_request_for_withdrawal<T: Config>(
        stash_account: &T::AccountId,
    ) -> DispatchResultWithPostInfo {
        ensure!(
            <StakingCount<T>>::contains_key(stash_account.clone()),
            Error::<T>::StashAccountNotFound
        );

        let controller_account = T::StakingHelper::get_controller_account(stash_account)?;

        // unlock operation is allowed ONLY for locked funds.
        ensure!(
            T::StakingHelper::are_funds_locked(&controller_account),
            Error::<T>::FundsAtUnlockedState
        );

        Ok(().into())
    }

    /// It includes:
    /// 1. Check if the pub key is a controller.
    /// 2. Checking the number of outpoints owned by the given pub key
    /// 3. Checking each outpoints if they are indeed owned by the pub key
    /// Returns a Result with an empty Ok, or an Err in string.
    /// # Arguments
    /// * `stash_account` - An existing stash account
    /// * `outpoints` - List of keys of unlocked utxos said to be "owned" by the controller_pubkey
    pub fn validate_withdrawal<T: Config>(
        stash_account: &T::AccountId,
        outpoints: &Vec<H256>,
    ) -> Result<ValidTransaction, &'static str> {
        ensure!(
            <StakingCount<T>>::contains_key(stash_account),
            Error::<T>::StashAccountNotFound
        );

        let (num_of_utxos, _) = <StakingCount<T>>::get(stash_account.clone())
            .ok_or("cannot find the stash account inside the StakingCount storage")?;
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
            utils::is_owned_locked_utxo::<T>(&utxo, stash_account)?;
        }

        let controller_account = T::StakingHelper::get_controller_account(stash_account)?;
        // if the funds are already unlocked, it means you should withdraw them first,
        // then perform a `LockForStaking`.
        ensure!(
            !T::StakingHelper::are_funds_locked(&controller_account),
            "Funds are still locked. Perform `unlock_request_for_withdrawal` first."
        );

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

mod utils {
    use super::*;
    use sp_runtime::DispatchError;

    pub fn is_owned_locked_utxo<T: Config>(
        utxo: &TransactionOutput<T::AccountId>,
        expected_stash_account: &T::AccountId,
    ) -> Result<(), &'static str> {
        match &utxo.destination {
            Destination::LockForStaking {
                stash_account,
                controller_account: _,
                session_key: _,
            }
            | Destination::LockExtraForStaking {
                stash_account,
                controller_account: _,
            } => {
                ensure!(
                    stash_account == expected_stash_account,
                    "hash of stake not owned"
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
        stash_account: &T::AccountId,
    ) -> DispatchResultWithPostInfo {
        log::debug!("Locking utxo({:?}) of stash {:?}", hash_key, stash_account);
        let (num_of_utxos, total) = <StakingCount<T>>::get(stash_account).unwrap_or((0, 0));
        <StakingCount<T>>::insert(
            stash_account.clone(),
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
