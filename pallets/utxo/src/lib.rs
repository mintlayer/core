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

#![cfg_attr(not(feature = "std"), no_std)]

pub use header::*;
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod header;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
    use core::marker::PhantomData;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};

    use crate::{validate_header, SignatureMethod, TXOutputHeader, TXOutputHeaderImpls, TokenType};
    use codec::{Decode, Encode};
    use frame_support::{
        dispatch::{DispatchResultWithPostInfo, Vec},
        pallet_prelude::*,
        sp_io::crypto,
        sp_runtime::traits::{BlakeTwo256, Dispatchable, Hash, SaturatedConversion},
        traits::IsSubType,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::{
        sp_std::collections::btree_map::BTreeMap,
        sr25519::{Public as SR25Pub, Signature as SR25Sig},
        H256, H512,
    };

    pub type Value = u128;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// runtime configuration
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The overarching call type.
        type Call: Dispatchable + From<Call<Self>> + IsSubType<Call<Self>> + Clone;

        type WeightInfo: WeightInfo;

        fn authorities() -> Vec<H256>;
    }

    pub trait WeightInfo {
        fn spend(u: u32) -> Weight;
    }

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash, Default,
    )]
    pub struct TransactionInput {
        pub(crate) outpoint: H256,
        pub(crate) sig_script: H512,
    }

    impl TransactionInput {
        pub fn new(outpoint: H256, sig_script: H512) -> Self {
            Self {
                outpoint,
                sig_script,
            }
        }
    }

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash, Default,
    )]
    pub struct TransactionOutput {
        pub(crate) value: Value,
        pub(crate) pub_key: H256,
        pub(crate) header: TXOutputHeader,
    }

    impl TransactionOutput {
        /// By default the header is 0:
        /// token type for both the value and fee is MLT,
        /// and the signature method is BLS.
        /// functions are available in TXOutputHeaderImpls to update the header.
        pub fn new(value: Value, pub_key: H256) -> Self {
            Self {
                value,
                pub_key,
                header: 0,
            }
        }
    }

    impl TXOutputHeaderImpls for TransactionOutput {
        fn set_token_type(&mut self, value_token_type: TokenType) {
            TokenType::insert(&mut self.header, value_token_type);
        }

        fn set_signature_method(&mut self, signature_method: SignatureMethod) {
            SignatureMethod::insert(&mut self.header, signature_method);
        }

        fn get_token_type(&self) -> Result<TokenType, &'static str> {
            TokenType::extract(self.header)
        }

        fn get_signature_method(&self) -> Result<SignatureMethod, &'static str> {
            SignatureMethod::extract(self.header)
        }

        fn validate_header(&self) -> Result<(), &'static str> {
            validate_header(self.header)
        }
    }

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash, Default)]
    pub struct Transaction {
        pub(crate) inputs: Vec<TransactionInput>,
        pub(crate) outputs: Vec<TransactionOutput>,
    }

    #[pallet::storage]
    #[pallet::getter(fn reward_total)]
    pub(super) type RewardTotal<T> = StorageValue<_, Value, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn utxo_store)]
    pub(super) type UtxoStore<T: Config> =
        StorageMap<_, Blake2_256, H256, Option<TransactionOutput>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TransactionSuccess(Transaction),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block_num: T::BlockNumber) {
            disperse_reward::<T>(&T::authorities(), block_num)
        }
    }

    // Strips a transaction of its Signature fields by replacing value with ZERO-initialized fixed hash.
    pub fn get_simple_transaction(tx: &Transaction) -> Vec<u8> {
        let mut trx = tx.clone();
        for input in trx.inputs.iter_mut() {
            input.sig_script = H512::zero();
        }

        trx.encode()
    }

    fn disperse_reward<T: Config>(auths: &[H256], block_number: T::BlockNumber) {
        let reward = <RewardTotal<T>>::take();
        let share_value: Value = reward
            .checked_div(auths.len() as Value)
            .ok_or("No authorities")
            .unwrap();
        if share_value == 0 {
            //put reward back if it can't be split nicely
            <RewardTotal<T>>::put(reward as Value);
            return;
        }

        let remainder = reward
            .checked_sub(share_value * auths.len() as Value)
            .ok_or("Sub underflow")
            .unwrap();

        log::debug!("disperse_reward:: reward total: {:?}", remainder);
        <RewardTotal<T>>::put(remainder as Value);

        for authority in auths {
            // TODO: where do we get the header info?
            // TODO: are the rewards always of MLT token type?
            let utxo = TransactionOutput::new(share_value, *authority);

            let hash = {
                let b_num = block_number.saturated_into::<u64>();
                BlakeTwo256::hash_of(&(&utxo, b_num))
            };

            if !<UtxoStore<T>>::contains_key(hash) {
                <UtxoStore<T>>::insert(hash, Some(utxo));
            }
        }
    }

    pub fn validate_transaction<T: Config>(
        tx: &Transaction,
    ) -> Result<ValidTransaction, &'static str> {
        //ensure rather than assert to avoid panic
        //both inputs and outputs should contain at least 1 utxo
        ensure!(!tx.inputs.is_empty(), "no inputs");
        ensure!(!tx.outputs.is_empty(), "no outputs");

        //ensure each input is used only a single time
        //maps each input into btree
        //if map.len() > num of inputs then fail
        //https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
        //WARNING workshop code has a bug here
        //https://github.com/substrate-developer-hub/utxo-workshop/blob/workshop/runtime/src/utxo.rs
        //input_map.len() > transaction.inputs.len() //THIS IS WRONG
        {
            let input_map: BTreeMap<_, ()> = tx.inputs.iter().map(|input| (input, ())).collect();
            //we want map size and input size to be equal to ensure each is used only once
            ensure!(
                input_map.len() == tx.inputs.len(),
                "each input should be used only once"
            );
        }
        //ensure each output is unique
        //map each output to btree to count unique elements
        //WARNING example code has a bug here
        //out_map.len() != transaction.outputs.len() //THIS IS WRONG
        {
            let out_map: BTreeMap<_, ()> = tx.outputs.iter().map(|output| (output, ())).collect();
            //check each output is defined only once
            ensure!(
                out_map.len() == tx.outputs.len(),
                "each output should be used once"
            );
        }

        let mut total_input: Value = 0;
        let mut total_output: Value = 0;
        let mut output_index: u64 = 0;
        let simple_tx = get_simple_transaction(tx);

        // In order to avoid race condition in network we maintain a list of required utxos for a tx
        // Example of race condition:
        // Assume both alice and bob have 10 coins each and bob owes charlie 20 coins
        // In order to pay charlie alice must first send 10 coins to bob which creates a new utxo
        // If bob uses the new utxo to try and send the coins to charlie before charlie receives the alice to bob 10 coins utxo
        // then the tx from bob to charlie is invalid. By maintaining a list of required utxos we can ensure the tx can happen as and
        // when the utxo is available. We use max longevity at the moment. That should be fixed.

        let mut missing_utxos = Vec::new();
        let mut new_utxos = Vec::new();
        let mut reward = 0;

        // Check that inputs are valid
        for input in tx.inputs.iter() {
            if let Some(input_utxo) = <UtxoStore<T>>::get(&input.outpoint) {
                ensure!(
                    crypto::sr25519_verify(
                        &SR25Sig::from_raw(*input.sig_script.as_fixed_bytes()),
                        &simple_tx,
                        &SR25Pub::from_h256(input_utxo.pub_key)
                    ),
                    "signature must be valid"
                );
                total_input = total_input
                    .checked_add(input_utxo.value)
                    .ok_or("input value overflow")?;
            } else {
                missing_utxos.push(input.outpoint.clone().as_fixed_bytes().to_vec());
            }
        }

        // Check that outputs are valid
        for output in tx.outputs.iter() {
            ensure!(output.value > 0, "output value must be nonzero");
            let hash = BlakeTwo256::hash_of(&(&tx.encode(), output_index));
            output_index = output_index.checked_add(1).ok_or("output index overflow")?;
            ensure!(!<UtxoStore<T>>::contains_key(hash), "output already exists");

            // Check the header is valid
            let res = output.validate_header();
            if let Err(e) = res {
                log::error!("Header error: {}", e);
            }
            ensure!(res.is_ok(), "header error. Please check the logs.");

            // checked add bug in example cod where it uses checked_sub
            total_output = total_output
                .checked_add(output.value)
                .ok_or("output value overflow")?;
            new_utxos.push(hash.as_fixed_bytes().to_vec());
        }

        // if no race condition, check the math
        if missing_utxos.is_empty() {
            ensure!(
                total_input >= total_output,
                "output value must not exceed input value"
            );
            reward = total_input
                .checked_sub(total_output)
                .ok_or("reward underflow")?;
        }

        Ok(ValidTransaction {
            priority: reward as u64,
            requires: missing_utxos,
            provides: new_utxos,
            longevity: TransactionLongevity::MAX,
            propagate: true,
        })
    }

    /// Update storage to reflect changes made by transaction
    /// Where each utxo key is a hash of the entire transaction and its order in the TransactionOutputs vector
    pub fn update_storage<T: Config>(
        tx: &Transaction,
        reward: Value,
    ) -> DispatchResultWithPostInfo {
        // Calculate new reward total
        let new_total = <RewardTotal<T>>::get()
            .checked_add(reward)
            .ok_or("Reward overflow")?;

        log::debug!("update_storage:: reward total: {:?}", new_total);
        <RewardTotal<T>>::put(new_total);

        // Removing spent UTXOs
        for input in &tx.inputs {
            log::debug!("removing {:?} in UtxoStore.", input.outpoint);
            <UtxoStore<T>>::remove(input.outpoint);
        }

        let mut index: u64 = 0;
        for output in &tx.outputs {
            let hash = BlakeTwo256::hash_of(&(&tx.encode(), index));
            index = index.checked_add(1).ok_or("output index overflow")?;
            log::debug!("inserting to UtxoStore {:?} as key {:?}", output, hash);
            <UtxoStore<T>>::insert(hash, Some(output));
        }

        Ok(().into())
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::spend(tx.inputs.len().saturating_add(tx.outputs.len()) as u32))]
        pub fn spend(_origin: OriginFor<T>, tx: Transaction) -> DispatchResultWithPostInfo {
            let tx_validity = validate_transaction::<T>(&tx)?;
            ensure!(tx_validity.requires.is_empty(), "missing inputs");

            update_storage::<T>(&tx, tx_validity.priority as Value)?;

            Self::deposit_event(Event::<T>::TransactionSuccess(tx));
            Ok(().into())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub genesis_utxos: Vec<TransactionOutput>,
        pub _marker: PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                genesis_utxos: vec![],
                _marker: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            self.genesis_utxos.iter().cloned().for_each(|u| {
                UtxoStore::<T>::insert(BlakeTwo256::hash_of(&u), Some(u));
            });
        }
    }
}
