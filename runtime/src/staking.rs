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

use crate::Perbill;

use codec::Decode;
use frame_support::dispatch::{DispatchResult, DispatchResultWithPostInfo, Vec};
use frame_support::fail;
use frame_system::{Config as SysConfig, RawOrigin};
use pallet_staking::{BalanceOf, Pallet as StakingPallet};
use pallet_utxo::staking::StakingHelper;
use sp_core::sp_std::vec;
use sp_runtime::traits::StaticLookup;

type StakeAccountId<T> = <T as SysConfig>::AccountId;
type LookupSourceOf<T> = <<T as SysConfig>::Lookup as StaticLookup>::Source;

pub struct StakeOps<T>(sp_core::sp_std::marker::PhantomData<T>);

impl<T: pallet_staking::Config + pallet_utxo::Config + pallet_session::Config> StakeOps<T>
where
    StakeAccountId<T>: From<[u8; 32]>,
    BalanceOf<T>: From<u128>,
{
    fn get_stash_account(
        controller_account: StakeAccountId<T>,
    ) -> Result<StakeAccountId<T>, pallet_utxo::Error<T>> {
        match <StakingPallet<T>>::ledger(controller_account.clone()) {
            None => {
                log::debug!("check sync with pallet-staking.");
                return Err(pallet_utxo::Error::<T>::ControllerAccountNotFound)?;
            }
            Some(stake_ledger) => Ok(stake_ledger.stash),
        }
    }

    fn bond(
        controller_account: StakeAccountId<T>,
        stash_account: StakeAccountId<T>,
        value: pallet_utxo::tokens::Value,
    ) -> DispatchResult {
        let controller_lookup: LookupSourceOf<T> = T::Lookup::unlookup(controller_account.clone());
        let reward_destination = pallet_staking::RewardDestination::Staked;

        // bond the funds
        StakingPallet::<T>::bond(
            RawOrigin::Signed(stash_account.clone()).into(),
            controller_lookup,
            value.into(),
            reward_destination,
        )
    }

    fn unbond(controller_account: StakeAccountId<T>) -> DispatchResult {
        let stake_ledger = <StakingPallet<T>>::ledger(controller_account.clone())
            .ok_or(pallet_utxo::Error::<T>::ControllerAccountNotFound)?;

        StakingPallet::<T>::unbond(
            RawOrigin::Signed(controller_account).into(),
            stake_ledger.total,
        )
    }

    fn set_session_keys(
        controller_account: StakeAccountId<T>,
        session_key: &Vec<u8>,
    ) -> DispatchResult {
        // session keys
        let sesh_key = <T as pallet_session::Config>::Keys::decode(&mut &session_key[..])
            .expect("SessionKeys decoded successfully");
        pallet_session::Pallet::<T>::set_keys(
            RawOrigin::Signed(controller_account).into(),
            sesh_key,
            vec![],
        )
    }

    fn apply_for_validator_role(controller_account: StakeAccountId<T>) -> DispatchResult {
        let validator_prefs = pallet_staking::ValidatorPrefs {
            commission: Perbill::from_percent(0),
            ..Default::default()
        };

        // applying for the role of "validator".
        StakingPallet::<T>::validate(
            RawOrigin::Signed(controller_account).into(),
            validator_prefs,
        )
    }
}

impl<T: pallet_staking::Config + pallet_utxo::Config + pallet_session::Config>
    StakingHelper<T::AccountId> for StakeOps<T>
where
    StakeAccountId<T>: From<[u8; 32]>,
    BalanceOf<T>: From<u128>,
{
    fn get_controller_account(
        stash_account: &StakeAccountId<T>,
    ) -> Result<StakeAccountId<T>, &'static str> {
        <StakingPallet<T>>::bonded(stash_account.clone())
            .ok_or(pallet_utxo::Error::<T>::StashAccountNotFound.into())
    }

    fn is_controller_account_exist(controller_account: &StakeAccountId<T>) -> bool {
        Self::get_stash_account(controller_account.clone()).is_ok()
    }

    fn can_decode_session_key(session_key: &Vec<u8>) -> bool {
        <T as pallet_session::Config>::Keys::decode(&mut &session_key[..]).is_ok()
    }

    fn are_funds_locked(controller_account: &StakeAccountId<T>) -> bool {
        // Information of unlocked funds are found in the `pallet-staking` ledger.
        // The ledger is stored as a map, with the controller_account as the key.
        match <StakingPallet<T>>::ledger(controller_account.clone()) {
            None => {
                log::error!("Controller account {:?} not found", controller_account);
            }
            Some(stake_ledger) => {
                if stake_ledger.unlocking.is_empty() {
                    return true;
                } else if stake_ledger.unlocking.len() > 1 {
                    log::error!(
                        "Pallet-staking ledger's unlocking field should only contain ONE element."
                    );
                }
            }
        }
        false
    }

    fn check_accounts_matched(
        controller_account: &StakeAccountId<T>,
        stash_account: &StakeAccountId<T>,
    ) -> bool {
        if let Ok(ledger_stash_acc) = Self::get_stash_account(controller_account.clone()) {
            if stash_account == &ledger_stash_acc {
                return true;
            }
        }
        log::error!(
            "Make sure to match correctly the stash account {:?} with the controller account.",
            stash_account
        );

        false
    }

    fn lock_for_staking(
        stash_account: &StakeAccountId<T>,
        controller_account: &StakeAccountId<T>,
        session_key: &Vec<u8>,
        value: u128,
    ) -> DispatchResultWithPostInfo {
        Self::bond(controller_account.clone(), stash_account.clone(), value)?;
        Self::set_session_keys(controller_account.clone(), session_key)?;
        Self::apply_for_validator_role(controller_account.clone())?;

        Ok(().into())
    }

    fn lock_extra_for_staking(
        stash_account: &StakeAccountId<T>,
        value: u128,
    ) -> DispatchResultWithPostInfo {
        StakingPallet::<T>::bond_extra(
            RawOrigin::Signed(stash_account.clone()).into(),
            value.into(),
        )?;

        Ok(().into())
    }

    fn unlock_request_for_withdrawal(
        stash_account: &StakeAccountId<T>,
    ) -> DispatchResultWithPostInfo {
        // get the controller account, given the stash_account.
        let controller_account = <StakingPallet<T>>::bonded(stash_account.clone())
            .ok_or(pallet_utxo::Error::<T>::StashAccountNotFound)?;

        // stop validating / block producing
        StakingPallet::<T>::chill(RawOrigin::Signed(controller_account.clone()).into())?;

        // unbond
        Self::unbond(controller_account)?;

        Ok(().into())
    }

    fn withdraw(stash_account: &StakeAccountId<T>) -> DispatchResultWithPostInfo {
        // get the controller account, given the stash_account.
        let controller_account = <StakingPallet<T>>::bonded(stash_account.clone())
            .ok_or(pallet_utxo::Error::<T>::StashAccountNotFound)?;

        let res = StakingPallet::<T>::withdraw_unbonded(
            RawOrigin::Signed(controller_account.clone()).into(),
            0,
        )?;

        // if the staking still exists, withdrawal was unsuccessful.
        if <StakingPallet<T>>::ledger(controller_account).is_some() {
            log::error!("no withdrawal was done.");
            fail!(pallet_utxo::Error::<T>::InvalidOperation)
        }

        Ok(res)
    }
}
