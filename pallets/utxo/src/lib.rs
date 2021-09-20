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
pub use script::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod header;
mod script;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
    use crate::TXOutputHeader;
    use crate::{OutputHeader, OutputHeaderHelper, TokenID};
    use crate::{ScriptPubKey, ScriptType};
    use codec::{Decode, Encode};
    use core::marker::PhantomData;
    use frame_support::{
        dispatch::{DispatchResultWithPostInfo, Vec},
        pallet_prelude::*,
        sp_io::crypto,
        sp_runtime::traits::{BlakeTwo256, Dispatchable, Hash, SaturatedConversion},
        traits::IsSubType,
    };
    use frame_system::pallet_prelude::*;
    use pallet_utxo_tokens::TokenListData;
    use pp_api::ProgrammablePoolApi;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use sp_core::{
        sp_std::collections::btree_map::BTreeMap,
        sr25519::{Public as SR25Pub, Signature as SR25Sig},
        H256, H512,
    };
    use sp_runtime::traits::{
        AtLeast32Bit, Zero, /*, StaticLookup , AtLeast32BitUnsigned, Member, One */
    };

    pub type Value = u128;
    pub type String = Vec<u8>;

    #[pallet::error]
    pub enum Error<T> {
        /// Account balance must be greater than or equal to the transfer amount.
        BalanceLow,
        /// Balance should be non-zero.
        BalanceZero,
        /// The signing account has no permission to do the operation.
        NoPermission,
        /// The given asset ID is unknown.
        Unknown,
        /// The origin account is frozen.
        Frozen,
        /// The asset ID is already taken.
        InUse,
        /// Invalid witness data given.
        BadWitness,
        /// Minimum balance should be non-zero.
        MinBalanceZero,
        /// No provider reference exists to allow a non-zero balance of a non-self-sufficient asset.
        NoProvider,
        /// Invalid metadata given.
        BadMetadata,
        /// No approval exists that would allow the transfer.
        Unapproved,
        /// The source account would not survive the transfer and it needs to stay alive.
        WouldDie,
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// runtime configuration
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type AssetId: Parameter + AtLeast32Bit + Default + Copy;

        /// The overarching call type.
        type Call: Dispatchable + From<Call<Self>> + IsSubType<Call<Self>> + Clone;

        type WeightInfo: WeightInfo;

        type ProgrammablePool: ProgrammablePoolApi<AccountId = Self::AccountId>;

        fn authorities() -> Vec<H256>;
    }

    pub trait WeightInfo {
        fn spend(u: u32) -> Weight;
        fn tokens_create(u: u32) -> Weight;
        fn tokens_spend(u: u32) -> Weight;
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
        pub(crate) script: ScriptPubKey,
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
                script: ScriptPubKey::new(),
            }
        }

        pub fn new_tokens(token_id: TokenID, value: Value, pub_key: H256) -> Self {
            let mut header = OutputHeader::new(0);
            header.set_token_id(token_id);
            let header = header.as_u128();
            Self {
                value,
                pub_key,
                header,
                script: ScriptPubKey::new(),
            }
        }
    }

    impl TransactionOutput {
        fn validate_header(&self) -> Result<(), &'static str> {
            // Check signature and token id
            self.header
                .as_tx_output_header()
                .validate()
                .then(|| ())
                .ok_or("Incorrect header")
        }
    }

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash, Default)]
    pub struct Transaction {
        pub(crate) inputs: Vec<TransactionInput>,
        pub(crate) outputs: Vec<TransactionOutput>,
    }

    #[pallet::storage]
    #[pallet::getter(fn token_list)]
    pub(super) type TokenList<T> = StorageValue<_, TokenListData, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn tokens_higher_id)]
    pub(super) type TokensHigherID<T> = StorageValue<_, TokenID, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn reward_total)]
    pub(super) type RewardTotal<T> = StorageValue<_, Value, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn utxo_store)]
    pub(super) type UtxoStore<T: Config> =
        StorageMap<_, Blake2_256, H256, Option<TransactionOutput>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(
        T::AccountId = "AccountId",    //     T::Balance = "Balance",
        T::AssetId = "AssetId"
    )]
    pub enum Event<T: Config> {
        /// Some asset class was created. \[asset_id, creator and owner\]
        TokenCreated(u32, T::AccountId),
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
        let share_value: Value =
            reward.checked_div(auths.len() as Value).ok_or("No authorities").unwrap();
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

    pub fn create<T: Config>(caller: &T::AccountId, code: &Vec<u8>, data: &Vec<u8>) {
        let weight: Weight = 6000000000;

        match T::ProgrammablePool::create(caller, weight, code, data) {
            Ok(_) => log::info!("success!"),
            Err(e) => log::error!("failure: {:#?}", e),
        }
    }

    pub fn call<T: Config>(caller: &T::AccountId, dest: &T::AccountId, data: &Vec<u8>) {
        let weight: Weight = 6000000000;

        match T::ProgrammablePool::call(caller, dest, weight, data) {
            Ok(_) => log::info!("success!"),
            Err(e) => log::error!("failure: {:#?}", e),
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

        let input_vec: Vec<(crate::TokenID, Value)> = tx
            .inputs
            .iter()
            .filter_map(|input| <UtxoStore<T>>::get(&input.outpoint))
            .map(|output| (OutputHeader::new(output.header).token_id(), output.value))
            .collect();

        let out_vec: Vec<(crate::TokenID, Value)> = tx
            .outputs
            .iter()
            .map(|output| (OutputHeader::new(output.header).token_id(), output.value))
            .collect();

        // If this is token creation call, then in out_vec we will have a token that doesn't registered yet

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
            } else {
                missing_utxos.push(input.outpoint.clone().as_fixed_bytes().to_vec());
            }
        }

        // Check that outputs are valid
        for output in tx.outputs.iter() {
            // Check the header is valid
            let res = output.validate_header();
            if let Err(e) = res {
                log::error!("Header error: {}", e);
            }
            ensure!(res.is_ok(), "header error. Please check the logs.");

            match output.script.stype {
                ScriptType::P2pkh => {
                    ensure!(output.value > 0, "output value must be nonzero");
                    let hash = BlakeTwo256::hash_of(&(&tx, output_index));
                    output_index = output_index.checked_add(1).ok_or("output index overflow")?;
                    ensure!(!<UtxoStore<T>>::contains_key(hash), "output already exists");
                    new_utxos.push(hash.as_fixed_bytes().to_vec());
                }
                ScriptType::OpCreate => {
                    log::info!("TODO validate OP_CREATE");
                }
                ScriptType::OpCall => {
                    log::info!("TODO validate OP_CALL");
                }
            }
        }

        // if no race condition, check the math
        if missing_utxos.is_empty() {
            // We have to check sum of input tokens is less or equal to output tokens.
            let mut inputs_sum: BTreeMap<TokenID, Value> = BTreeMap::new();
            let mut outputs_sum: BTreeMap<TokenID, Value> = BTreeMap::new();

            for x in input_vec {
                let value =
                    x.1.checked_add(*inputs_sum.get(&x.0).unwrap_or(&0))
                        .ok_or("input value overflow")?;
                inputs_sum.insert(x.0, value);
            }
            for x in out_vec {
                let value =
                    x.1.checked_add(*outputs_sum.get(&x.0).unwrap_or(&0))
                        .ok_or("output value overflow")?;
                outputs_sum.insert(x.0, value);
            }

            for output_token in &outputs_sum {
                match inputs_sum.get(&output_token.0) {
                    Some(input_value) => ensure!(
                        input_value >= &output_token.1,
                        "output value must not exceed input value"
                    ),
                    None => frame_support::fail!("input for the token not found"),
                }
            }

            // Reward at the moment only in MLT
            reward = inputs_sum[&(crate::TokenType::MLT as TokenID)]
                .checked_sub(outputs_sum[&(crate::TokenType::MLT as TokenID)])
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
        caller: &T::AccountId,
        tx: &Transaction,
        reward: Value,
    ) -> DispatchResultWithPostInfo {
        // Calculate new reward total
        let new_total = <RewardTotal<T>>::get().checked_add(reward).ok_or("Reward overflow")?;

        log::debug!("update_storage:: reward total: {:?}", new_total);
        <RewardTotal<T>>::put(new_total);

        // Removing spent UTXOs
        for input in &tx.inputs {
            log::debug!("removing {:?} in UtxoStore.", input.outpoint);
            <UtxoStore<T>>::remove(input.outpoint);
        }

        let mut index: u64 = 0;
        for output in &tx.outputs {
            match output.script.stype {
                ScriptType::P2pkh => {
                    let hash = BlakeTwo256::hash_of(&(&tx, index));
                    index = index.checked_add(1).ok_or("output index overflow")?;
                    log::debug!("inserting to UtxoStore {:?} as key {:?}", output, hash);
                    <UtxoStore<T>>::insert(hash, Some(output));
                }
                ScriptType::OpCreate => {
                    create::<T>(caller, &output.script.script, &output.script.data);
                }
                ScriptType::OpCall => {
                    // TODO convert pubkey of tx to the destination (contract transaction), fix this
                    let mut tmp = output.pub_key.as_bytes().clone();
                    let id = T::AccountId::decode(&mut tmp).unwrap();
                    call::<T>(caller, &id, &output.script.data);
                }
            }
        }

        Ok(().into())
    }

    pub fn spend<T: Config>(caller: &T::AccountId, tx: &Transaction) -> DispatchResultWithPostInfo {
        let tx_validity = validate_transaction::<T>(tx)?;
        ensure!(tx_validity.requires.is_empty(), "missing inputs");
        update_storage::<T>(&caller, tx, tx_validity.priority as Value)?;
        Ok(().into())
    }

    pub fn tokens_create<T: Config>(
        caller: &T::AccountId,
        public: H256,
        input_for_fee: TransactionInput,
        token_name: String,
        token_ticker: String,
        supply: Value,
    ) -> DispatchResultWithPostInfo {
        ensure!(!supply.is_zero(), Error::<T>::MinBalanceZero);

        // Take a free TokenID
        let token_id =
            <TokensHigherID<T>>::get().checked_add(1).ok_or("All tokens IDs has taken")?;
        sp_runtime::print("TOKEN ID IS");
        sp_runtime::print(&token_id);

        // Input with MLT FEE
        let fee = UtxoStore::<T>::get(input_for_fee.outpoint).unwrap().value;
        ensure!(fee < 99, Error::<T>::Unapproved);

        // Save in UTXO
        let instance = crate::TokenInstance::new(token_id, token_name, token_ticker, supply);
        let mut tx = Transaction {
            inputs: crate::vec![
                // Fee an input equal 100 MLT
                input_for_fee,
            ],
            outputs: crate::vec![
                // Output a new tokens
                TransactionOutput::new_tokens(token_id, supply, public),
            ],
        };

        // We shall make an output to return odd funds
        if fee > 100 {
            tx.outputs.push(TransactionOutput::new(fee - 100, public));
        }

        // Save in Store
        <TokenList<T>>::mutate(|x| x.push(instance));

        // Success
        spend::<T>(caller, &tx)?;
        Ok(().into())
    }

    pub fn tokens_spend<T: Config>(
        caller: &T::AccountId,
        public: H256,
        inputs: Vec<TransactionInput>,
        // dest: <T::Lookup as StaticLookup>::Source,
        token_id: TokenID,
        amount: Value,
    ) -> DispatchResultWithPostInfo {
        // let dest = T::Lookup::lookup(dest)?;
        let tx = Transaction {
            inputs,
            outputs: crate::vec![
                // Output a new tokens
                TransactionOutput::new_tokens(token_id, amount, public),
            ],
        };
        spend::<T>(caller, &tx)?;
        Ok(().into())
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::spend(tx.inputs.len().saturating_add(tx.outputs.len()) as u32))]
        pub fn spend(origin: OriginFor<T>, tx: Transaction) -> DispatchResultWithPostInfo {
            spend::<T>(&ensure_signed(origin)?, &tx)?;
            Self::deposit_event(Event::<T>::TransactionSuccess(tx));
            Ok(().into())
        }

        #[pallet::weight(100)]
        //T::WeightInfo::tokens_create(input_for_fee.len().saturating_add(100) as u32))]
        pub fn tokens_create(
            origin: OriginFor<T>,
            public: H256,
            input_for_fee: TransactionInput,
            token_name: String,
            token_ticker: String,
            supply: Value,
        ) -> DispatchResultWithPostInfo {
            let caller = &ensure_signed(origin)?;
            tokens_create::<T>(
                caller,
                public,
                input_for_fee,
                token_name,
                token_ticker,
                supply,
            )?;
            Self::deposit_event(Event::<T>::TokenCreated(1u32, caller.clone()));
            Ok(().into())
        }

        #[pallet::weight(100)]
        // T::WeightInfo::tokens_spend(inputs.len().saturating_add(100) as u32))]
        pub fn tokens_spend(
            origin: OriginFor<T>,
            public: H256,
            inputs: Vec<TransactionInput>,
            //dest: <T::Lookup as StaticLookup>::Source,
            token_id: TokenID,
            amount: Value,
        ) -> DispatchResultWithPostInfo {
            let caller = &ensure_signed(origin)?;
            tokens_spend::<T>(caller, public, inputs, token_id, amount)?;
            //Self::deposit_event(Event::<T>::TransactionSuccess(tx));
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

use frame_support::inherent::Vec;
use pallet_utxo_tokens::{TokenInstance, TokenListData};

impl<T: Config> crate::Pallet<T> {
    pub fn send() -> u32 {
        1337
    }

    pub fn tokens_list() -> TokenListData {
        <TokenList<T>>::get()
    }

    pub fn balance(_caller: &T::AccountId) -> Vec<TokenInstance> {
        Vec::new()
    }
}

use frame_support::pallet_prelude::DispatchResultWithPostInfo;
use sp_core::{
    crypto::UncheckedFrom,
    {H256, H512},
};
use sp_runtime::sp_std::vec;
use utxo_api::UtxoApi;

impl<T: Config> UtxoApi for Pallet<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    type AccountId = T::AccountId;

    fn spend(
        caller: &T::AccountId,
        value: u128,
        address: H256,
        utxo: H256,
        sig: H512,
    ) -> DispatchResultWithPostInfo {
        spend::<T>(
            caller,
            &Transaction {
                inputs: vec![crate::TransactionInput::new(utxo, sig)],
                outputs: vec![crate::TransactionOutput::new(value, address)],
            },
        )
    }
}
