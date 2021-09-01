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

use contract_provider::ContractProvider;
use frame_support::{dispatch::Vec, weights::Weight};
use sp_core::{crypto::UncheckedFrom, Bytes};

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::{H256, H512};
    use utxo_api::UtxoApi;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_contracts::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Utxo: UtxoApi<AccountId = Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

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

impl<T: Config> ContractProvider for Pallet<T>
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

        let _ = pallet_contracts::Pallet::<T>::bare_instantiate(
            caller.clone(),
            endowment * 100_u32.into(), // TODO
            gas_limit,
            code,
            data.to_vec(),
            Vec::new(),
            false, // calculate rent projection
            true,  // enable debugging
        );

        Ok(())
    }

    fn call(
        caller: &T::AccountId,
        dest: &T::AccountId,
        gas_limit: Weight,
        input_data: &Vec<u8>,
    ) -> Result<(), &'static str> {
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
