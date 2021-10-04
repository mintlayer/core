use crate::Perbill;

use codec::{Decode, EncodeLike};
use frame_support::{traits::Currency, dispatch::{DispatchResultWithPostInfo, Vec}};
use frame_system::{Config as SysConfig, RawOrigin};
use pallet_staking::Pallet as StakingPallet;
use pallet_utxo::staking::StakingHelper;
use sp_core::{H256, sp_std::vec, sr25519::Public};
use sp_runtime::traits::StaticLookup;
use sp_runtime::AccountId32;

type StakeAccountId<T> =  <T as SysConfig>::AccountId;
type LookupSourceOf<T> = <<T as SysConfig>::Lookup as StaticLookup>::Source;

pub struct StakeOps<T>(sp_core::sp_std::marker::PhantomData<T>);
impl <T: pallet_staking::Config + pallet_utxo::Config + pallet_session::Config> StakingHelper<T::AccountId> for StakeOps<T>
    where StakeAccountId<T>: From<Public> + EncodeLike<AccountId32>
{
    fn get_account_id(pub_key: &H256) -> StakeAccountId<T> {
      Public::from_h256(pub_key.clone()).into()
    }

    fn stake(stash_account: &StakeAccountId<T>, controller_account: &StakeAccountId<T>, rotate_keys: &mut Vec<u8>) -> DispatchResultWithPostInfo {
        let controller_lookup: LookupSourceOf<T> = T::Lookup::unlookup(controller_account.clone());
        let amount = T::Currency::minimum_balance();
        let reward_destination = pallet_staking::RewardDestination::Staked;

        // bond the funds
        StakingPallet::<T>::bond(
            RawOrigin::Signed(stash_account.clone()).into(),
            controller_lookup,
            amount,
            reward_destination
        )?;

        let rotate_keys = sp_core::Bytes::from(rotate_keys.to_vec());
        // session keys
        let sesh_key = <T as pallet_session::Config>::Keys::decode(&mut &rotate_keys[..]).expect("SessionKeys decoded successfully");
        pallet_session::Pallet::<T>::set_keys(
            RawOrigin::Signed(controller_account.clone()).into(),
            sesh_key,
            vec![]
        )?;

        let validator_prefs = pallet_staking::ValidatorPrefs {
            commission: Perbill::from_percent(0),
            ..Default::default()
        };

        // validate
        StakingPallet::<T>::validate(
            RawOrigin::Signed(controller_account.clone()).into(),
            validator_prefs
        )?;

        Ok(().into())
    }

    fn pause(controller_account: &StakeAccountId<T>) -> DispatchResultWithPostInfo {
        // stop validating / block producing
        StakingPallet::<T>::chill(RawOrigin::Signed(controller_account.clone()).into())?;

        // get the total balance to free up
        if let Some(stake_ledger) = <StakingPallet<T>>::ledger(controller_account.clone()) {

            // let balance:BalanceOf<T> = stake_ledger.total;
            // unbond
            StakingPallet::<T>::unbond(
                RawOrigin::Signed(controller_account.clone()).into(),
                stake_ledger.total
            )?;
        } else {
            log::error!("check sync with pallet-staking.");
            Err(pallet_utxo::Error::<T>::NoStakingRecordFound)?
        }

        Ok(().into())
    }


    fn withdraw(controller_account: &StakeAccountId<T>) -> DispatchResultWithPostInfo {
        StakingPallet::<T>::withdraw_unbonded(RawOrigin::Signed(controller_account.clone()).into(),0)?;

        Ok(().into())
    }
}