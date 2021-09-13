// Copyright (c) 2021 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://spdx.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Author(s): A. Altonen
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::sp_runtime::SaturatedConversion;
pub use frame_support::{
    construct_runtime,
    inherent::Vec,
    parameter_types,
    sp_runtime::DispatchError,
    traits::{
        Currency, Everything, IsSubType, KeyOwnerProofSystem, LockableCurrency, Nothing,
        Randomness, ReservableCurrency,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        Weight,
    },
    StorageValue,
};
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use pp_api::ProgrammablePoolApi;
use sp_core::{crypto::UncheckedFrom, Bytes, H256};

#[frame_support::pallet]
pub mod pallet {
    use frame_support::inherent::Vec;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::{H256, H512};
    use utxo_api::UtxoApi;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_contracts::Config + pallet_balances::Config
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Utxo: UtxoApi<AccountId = Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash, Default,
    )]
    pub struct ContractFundInfo<Balance> {
        pub balance: Balance,
        pub utxos: Vec<H256>,
    }

    #[pallet::storage]
    #[pallet::getter(fn fundable_contracts)]
    pub(super) type FundableContracts<T: Config> =
        StorageMap<_, Identity, T::AccountId, Option<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn contract_info)]
    pub(super) type ContractInfo<T: Config> =
        StorageMap<_, Identity, T::AccountId, Option<ContractFundInfo<T::Balance>>, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn spend(
            origin: OriginFor<T>,
            value: u128,
            address: H256,
            utxo: H256,
            sig: H512,
        ) -> DispatchResultWithPostInfo {
            T::Utxo::spend(&ensure_signed(origin)?, value, address, utxo, sig)
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub _marker: PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                _marker: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {}
    }
}

impl<T: Config> ProgrammablePoolApi for Pallet<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    type AccountId = T::AccountId;

    fn create(
        caller: &T::AccountId,
        gas_limit: Weight,
        code: &Vec<u8>,
        data: &Vec<u8>,
    ) -> Result<(), &'static str> {
        let code = pallet_contracts_primitives::Code::Upload(Bytes(code.to_vec()));
        let endowment = pallet_contracts::Pallet::<T>::subsistence_threshold();

        let addr = match pallet_contracts::Pallet::<T>::bare_instantiate(
            caller.clone(),
            endowment * 2u32.into(),
            gas_limit,
            code,
            data.to_vec(),
            Vec::new(),
            false, // calculate rent projection
            true,  // enable debugging
        )
        .result
        {
            Ok(v) => v.account_id,
            Err(e) => return Err("Failed to instantiate smart contract"),
        };

        // TODO: one level of indirection is needed here to allow the ownership of multiple smart contracts
        // TODO: add selector? or derive new address for funding?

        // contract owned by the caller (needed because contract is called using `caller` [it's a hack])
        <FundableContracts<T>>::insert(caller, Some(addr.clone()));

        // funding information of newly created contract
        //
        // TODO: save (value, utxo_hash) instead to allow coin picker algorithm to function more efficiently
        <ContractInfo<T>>::insert(
            addr,
            Some(ContractFundInfo {
                balance: 0u32.into(),
                utxos: Vec::new(),
            }),
        );

        Ok(())
    }

    fn call(
        caller: &T::AccountId,
        dest: &T::AccountId, // TODO this should be the fundable sub-address/selector
        gas_limit: Weight,
        utxo_hash: H256,
        utxo_value: u128,
        input_data: &Vec<u8>,
    ) -> Result<(), &'static str> {
        let acc_id = match <FundableContracts<T>>::get(&dest) {
            Some(v) => v,
            None => {
                log::error!("{:?} does not own any smart contracts!", caller);
                return Err("No contracts");
            }
        };

        <ContractInfo<T>>::mutate(&acc_id, |info| {
            info.as_mut().unwrap().balance += utxo_value.saturated_into();
            info.as_mut().unwrap().utxos.push(utxo_hash);
        });

        let res = pallet_contracts::Pallet::<T>::bare_call(
            caller.clone(),
            acc_id,
            0u32.into(),
            gas_limit,
            input_data.to_vec(),
            true, // enable debugging
        );

        Ok(())
    }
}
