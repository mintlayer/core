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

pub use frame_support::{
    construct_runtime,
    dispatch::Vec,
    parameter_types,
    sp_runtime::{DispatchError, SaturatedConversion},
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
    pub trait Config: frame_system::Config + pallet_contracts::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Utxo: UtxoApi<AccountId = Self::AccountId>;
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug)]
    pub struct ContractBalance {
        pub funds: u128,
        pub utxos: Vec<(H256, u128)>,
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn contract_balances)]
    pub(super) type ContractBalances<T: Config> =
        StorageMap<_, Identity, T::AccountId, Option<ContractBalance>, ValueQuery>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {}

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
        _utxo_hash: H256,
        _utxo_value: u128,
        data: &Vec<u8>,
    ) -> Result<(), &'static str> {
        let code = pallet_contracts_primitives::Code::Upload(Bytes(code.to_vec()));
        let endowment = pallet_contracts::Pallet::<T>::subsistence_threshold();

        let res = match pallet_contracts::Pallet::<T>::bare_instantiate(
            caller.clone(),
            endowment * 100_u32.into(), // TODO
            gas_limit,
            code,
            data.to_vec(),
            Vec::new(),
            true, // enable debugging
        )
        .result
        {
            Ok(res) => res,
            Err(e) => {
                log::error!("Failed to instantiate contract: {:?}", e);
                return Err("Failed to instantiate contract");
            }
        };

        // Create balance entry for the smart contract
        <ContractBalances<T>>::insert(
            res.account_id,
            Some(ContractBalance {
                funds: 0,
                utxos: Vec::new(),
            }),
        );

        Ok(())
    }

    fn call(
        caller: &T::AccountId,
        dest: &T::AccountId,
        gas_limit: Weight,
        utxo_hash: H256,
        utxo_value: u128,
        fund_contract: bool,
        input_data: &Vec<u8>,
    ) -> Result<(), &'static str> {
        // check if `dest` exist and if it does, update its balance information
        <ContractBalances<T>>::get(&dest).ok_or("Contract doesn't exist!")?;
        <ContractBalances<T>>::mutate(dest, |info| {
            info.as_mut().unwrap().utxos.push((utxo_hash, utxo_value));
        });

        // only if explicitly specified, fund the contract
        if fund_contract {
            <ContractBalances<T>>::mutate(dest, |info| {
                info.as_mut().unwrap().funds += utxo_value.saturated_into::<u128>();
            });
        }

        let value = pallet_contracts::Pallet::<T>::subsistence_threshold();
        let _ = pallet_contracts::Pallet::<T>::bare_call(
            caller.clone(),
            dest.clone(),
            value * 0u32.into(),
            gas_limit,
            input_data.to_vec(),
            true, // enable debugging
        );

        Ok(())
    }
}

impl<T: pallet_contracts::Config + pallet_balances::Config + pallet::Config> ChainExtension<T>
    for Pallet<T>
{
    fn call<E: Ext>(func_id: u32, _env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        log::error!("Called an unregistered `func_id`: {:}", func_id);
        return Err(DispatchError::Other("Unimplemented func_id"));
    }

    fn enabled() -> bool {
        true
    }
}
