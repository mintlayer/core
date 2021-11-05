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

use codec::{Decode, Encode};
pub use frame_support::{
    construct_runtime,
    dispatch::Vec,
    ensure, parameter_types,
    sp_runtime::{DispatchError, SaturatedConversion},
    traits::{
        Currency, Everything, IsSubType, KeyOwnerProofSystem, LockableCurrency, Nothing,
        Randomness, ReservableCurrency,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        Weight,
    },
    BoundedVec, StorageValue,
};
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use pp_api::ProgrammablePoolApi;
use sp_core::{crypto::UncheckedFrom, Bytes, H256};
use utxo_api::UtxoApi;

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

/// Create Pay-to-Pubkey transaction from smart contract's UTXOs
/// and send it by calling into the UTXO system.
///
/// UTXO system implements the consensus-critical coin-picking
/// algorithm by condensing all vins into one transaction and
/// using the outpoint in asceding order to select the place
/// for each vin in the array of inputs. This ensures that all
/// PP validator nodes that execute the transaction output the
/// exact same TX.
///
/// # Arguments
/// * `caller` - Smart contract's account id
/// * `dest`   - Recipients account id
/// * `value`  - How much is tranferred to `dest`
fn send_p2pk_tx<T: Config>(
    caller: &T::AccountId,
    dest: &T::AccountId,
    value: u128,
) -> Result<(), DispatchError> {
    let fund_info =
        <ContractBalances<T>>::get(&caller).ok_or(DispatchError::Other("Caller doesn't exist"))?;
    ensure!(fund_info.funds >= value, "Caller doesn't have enough funds");
    let outpoints = fund_info.utxos.iter().map(|x| x.0).collect::<Vec<H256>>();

    T::Utxo::submit_c2pk_tx(caller, dest, value, &outpoints).map(|_| {
        <ContractBalances<T>>::mutate(&caller, |info| {
            info.as_mut().unwrap().utxos = Vec::new();
            info.as_mut().unwrap().funds = 0;
        });
    })
}

/// Create Contract-to-Contract transfer that allows smart contracts to
/// call each other and transfer funds through the UTXO system.
///
/// UTXO system converts this high-level transaction request from
/// `caller` to `dest` to an actual transaction, uses OP_SPEND
/// to unlock the funds of the UTXOs that the `caller` has
/// acquired and creates a new vout with `data` that
/// calls `dest` and transfers all funds of `caller` to this smart contract
///
/// * `caller` -  Smart contract that is doing the calling
/// * `dest` - Smart contract that is to be called
/// * `data` - Selector and all other data `dest` takes as input
fn send_c2c_tx<T: Config>(
    caller: &T::AccountId,
    dest: &T::AccountId,
    data: &Vec<u8>,
) -> Result<(), DispatchError> {
    let fund_info = <ContractBalances<T>>::get(caller).ok_or(DispatchError::Other(
        "Contract doesn't own any UTXO or it doesn't exist!",
    ))?;
    let outpoints = fund_info.utxos.iter().map(|x| x.0).collect::<Vec<H256>>();

    T::Utxo::submit_c2c_tx(caller, dest, fund_info.funds, data, &outpoints).map(|_| {
        <ContractBalances<T>>::mutate(&caller, |info| {
            info.as_mut().unwrap().utxos = Vec::new();
            info.as_mut().unwrap().funds = 0;
        });
    })
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

        let res = pallet_contracts::Pallet::<T>::bare_instantiate(
            caller.clone(),
            endowment * 100_u32.into(), // TODO
            gas_limit,
            code,
            data.to_vec(),
            Vec::new(),
            true, // enable debugging
        )
        .result
        .map_err(|e| {
            log::error!("Instantation failed: {:?}", e);
            "Failed to instantiate smart contract"
        })?;

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
        input_data: &Vec<u8>,
    ) -> Result<(), &'static str> {
        let _ = pallet_contracts::Pallet::<T>::bare_call(
            caller.clone(),
            dest.clone(),
            0u32.into(),
            gas_limit,
            input_data.to_vec(),
            true, // enable debugging
        )
        .result
        .map_err(|e| {
            log::error!("Call failed: {:?}", e);
            "Failed to call smart contract"
        })?;

        Ok(())
    }

    fn fund(dest: &T::AccountId, utxo_hash: H256, utxo_value: u128) -> Result<(), &'static str> {
        <ContractBalances<T>>::get(&dest).ok_or("Contract doesn't exist!")?;
        <ContractBalances<T>>::mutate(dest, |info| {
            info.as_mut().unwrap().utxos.push((utxo_hash, utxo_value));
            info.as_mut().unwrap().funds += utxo_value;
        });

        Ok(())
    }
}

enum ChainExtensionCall {
    Transfer = 1000,
    Balance = 1001,
    Call = 1002,
}

impl<T: pallet_contracts::Config + pallet::Config> ChainExtension<T> for Pallet<T> {
    fn call<E: Ext>(func_id: u32, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        // Fetch AccountId of the caller from the ChainExtension's memory
        // This way the progrmmable pool can force the caller of the ChainExtension
        // to only spend their own funds as `ContractBalances` will be queried
        // using `acc_id` and user cannot control the value of this variable
        let mut env = env.buf_in_buf_out();
        let acc_id = env.ext().address().encode();
        let acc_id: T::AccountId = T::AccountId::decode(&mut &acc_id[..])
            .map_err(|_| "Failed to get smart contract's AccountId")?;

        match func_id {
            x if x == ChainExtensionCall::Transfer as u32 => {
                let (dest, value): (T::AccountId, u128) = env.read_as()?;

                if !<ContractBalances<T>>::get(&dest).is_none() {
                    return Err(DispatchError::Other(
                        "Contract-to-contract transactions not implemented",
                    ));
                }

                send_p2pk_tx::<T>(&acc_id, &dest, value)?
            }
            x if x == ChainExtensionCall::Balance as u32 => {
                let fund_info = <ContractBalances<T>>::get(&acc_id).ok_or(DispatchError::Other(
                    "Contract doesn't own any UTXO or it doesn't exist!",
                ))?;

                env.write(&fund_info.funds.encode(), false, None)
                    .map_err(|_| DispatchError::Other("Failed to return value?"))?;
            }
            x if x == ChainExtensionCall::Call as u32 => {
                // `read_as_unbounded()` has to be used here because the size of `data`
                //  is only known during runtime
                let (dest, selector, mut data): (T::AccountId, [u8; 4], Vec<u8>) =
                    env.read_as_unbounded(env.in_len())?;

                if <ContractBalances<T>>::get(&dest).is_none() {
                    return Err(DispatchError::Other("Destination doesn't exist"));
                }

                if acc_id == dest {
                    return Err(DispatchError::Other("Contract cannot call itself"));
                }

                // append data to the selector so the final data
                // passed on to the contract is in correct format
                let mut selector = selector.to_vec();
                selector.append(&mut data);

                // C2C transfers all funds as refunding to a contract is not possible (at least for now)
                send_c2c_tx::<T>(&acc_id, &dest, &selector)?
            }
            _ => {
                log::error!("Called an unregistered `func_id`: {:}", func_id);
                return Err(DispatchError::Other("Unimplemented function"));
            }
        }
        Ok(RetVal::Converging(0))
    }

    fn enabled() -> bool {
        true
    }
}
