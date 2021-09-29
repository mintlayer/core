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
// Author(s): C. Yap, L. Kuklinek

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
mod script;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
    use core::convert::TryInto;
    use core::marker::PhantomData;
    use hex_literal::hex;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};

    use crate::{validate_header, SignatureMethod, TXOutputHeader, TXOutputHeaderImpls, TokenType};
    use chainscript::Script;
    use codec::{Decode, Encode};
    use frame_support::{
        dispatch::{DispatchResultWithPostInfo, Vec},
        pallet_prelude::*,
        sp_io::crypto,
        sp_runtime::traits::{BlakeTwo256, Dispatchable, Hash, SaturatedConversion},
        traits::IsSubType,
    };
    use frame_system::pallet_prelude::*;
    use pp_api::ProgrammablePoolApi;
    use sp_core::sr25519::Public;
    use sp_core::{
        sp_std::collections::btree_map::BTreeMap,
        sp_std::{str, vec},
        sr25519::{Public as SR25Pub, Signature as SR25Sig},
        testing::SR25519, // TODO: ???
        H256,
        H512,
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

        type ProgrammablePool: ProgrammablePoolApi<AccountId = Self::AccountId>;

        fn authorities() -> Vec<H256>;
    }

    pub trait WeightInfo {
        fn spend(u: u32) -> Weight;
        fn send_to_pubkey(u: u32) -> Weight;
    }

    /// Transaction input
    ///
    /// The input contains two pieces of information used to unlock the funds being spent. The
    /// first one is `lock` and is usually committed to in UTXO specifed by the `outpoint`. It
    /// contains data used to protect the funds. The second one is `witness` that contains a proof
    /// that redeemer is allowed to spend the funds. The `witness` field does not contribute to the
    /// transaction ID hash to emulate the behaviour of SegWit.
    ///
    /// Both `lock` and `witness` are raw byte arrays. The exact interpretation depends on the
    /// [Destination] kind of the UTXO being spent. A couple of examples:
    ///
    /// * `Destination::Pubkey(key)`
    ///   * `lock` has to be empty
    ///   * `witness` contains the signature for the transaction and given pubkey
    /// * `Destination::ScriptHash(script_hash)`
    ///   * `lock` is the script fully expanded out, hash of `lock` has to match `script_hash`
    ///   * `witness` is a script that generates the input to the `lock` script
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash, Default,
    )]
    pub struct TransactionInput {
        /// The output being spent
        pub(crate) outpoint: H256,
        /// The lock data
        pub(crate) lock: Vec<u8>,
        /// The witness data
        pub(crate) witness: Vec<u8>,
    }

    impl TransactionInput {
        /// New input with a signature in the `witness` field.
        pub fn new_with_signature(outpoint: H256, sig_script: H512) -> Self {
            Self {
                outpoint,
                lock: Vec::new(),
                witness: (&sig_script[..]).to_vec(),
            }
        }

        /// New input with empty `lock` and `witness` to be filled later.
        pub fn new_empty(outpoint: H256) -> Self {
            Self {
                outpoint,
                lock: Vec::new(),
                witness: Vec::new(),
            }
        }

        /// New input with lock script and witness script.
        pub fn new_script(outpoint: H256, lock: Script, witness: Script) -> Self {
            Self {
                outpoint,
                lock: lock.into_bytes(),
                witness: witness.into_bytes(),
            }
        }

        /// Get lock hash.
        pub fn lock_hash(&self) -> H256 {
            BlakeTwo256::hash(&self.lock)
        }
    }

    /// Destination specifies where a payment goes. Can be a pubkey hash, script, etc.
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash)]
    pub enum Destination<AccountId> {
        /// Plain pay-to-pubkey
        Pubkey(H256),
        /// Pay to fund a new programmable pool. Takes code and data.
        CreatePP(Vec<u8>, Vec<u8>),
        /// Pay to an existing contract. Takes a destination account and input data.
        CallPP(AccountId, Vec<u8>),
        /// Pay to script hash
        ScriptHash(H256),
        /// Pay to pubkey hash
        PubkeyHash(Vec<u8>),
    }

    impl<AccountId> Destination<AccountId> {
        /// Hash of an empty byte array
        const EMPTY: H256 = H256(hex!(
            "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
        ));

        /// Calculate lock commitment for given destination.
        ///
        /// The `lock` field of the input spending the UTXO has to match this hash.
        pub fn lock_commitment(&self) -> &H256 {
            match self {
                Destination::ScriptHash(hash) => hash,
                _ => &Self::EMPTY,
            }
        }
    }

    /// Output of a transaction
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash)]
    pub struct TransactionOutput<AccountId> {
        pub(crate) value: Value,
        pub(crate) header: TXOutputHeader,
        pub(crate) destination: Destination<AccountId>,
    }

    impl<AccountId> TransactionOutput<AccountId> {
        /// By default the header is 0:
        /// token type for both the value and fee is MLT,
        /// and the signature method is BLS.
        /// functions are available in TXOutputHeaderImpls to update the header.
        pub fn new_pubkey(value: Value, pub_key: H256) -> Self {
            Self {
                value,
                header: 0,
                destination: Destination::Pubkey(pub_key),
            }
        }

        /// Create a new output to create a smart contract.
        pub fn new_create_pp(value: Value, code: Vec<u8>, data: Vec<u8>) -> Self {
            Self {
                value,
                header: 0,
                destination: Destination::CreatePP(code, data),
            }
        }

        /// Create a new output to call a smart contract routine.
        pub fn new_call_pp(value: Value, dest_account: AccountId, input: Vec<u8>) -> Self {
            Self {
                value,
                header: 0,
                destination: Destination::CallPP(dest_account, input),
            }
        }

        /// Create a new output to given script hash.
        pub fn new_script_hash(value: Value, hash: H256) -> Self {
            Self {
                value,
                header: 0,
                destination: Destination::ScriptHash(hash),
            }
        }

        /// Create a new output to given pubkey hash
        pub fn new_pubkey_hash(value: Value, script: Script) -> Self {
            Self {
                value,
                header: 0,
                destination: Destination::PubkeyHash(script.into_bytes()),
            }
        }
    }

    impl<AccountId> TXOutputHeaderImpls for TransactionOutput<AccountId> {
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
    pub struct Transaction<AccountId> {
        pub(crate) inputs: Vec<TransactionInput>,
        pub(crate) outputs: Vec<TransactionOutput<AccountId>>,
    }

    impl<AccountId> Transaction<AccountId> {
        /// Iterator over transaction outputs together with output indices
        pub fn enumerate_outputs(
            &self,
        ) -> Result<
            impl Iterator<Item = (u64, &TransactionOutput<AccountId>)> + ExactSizeIterator,
            &'static str,
        > {
            ensure!((self.outputs.len() as u32) < u32::MAX, "too many outputs");
            Ok(self.outputs.iter().enumerate().map(|(ix, out)| (ix as u64, out)))
        }
    }

    impl<AccountId: Encode> Transaction<AccountId> {
        /// Get hash of output at given index.
        pub fn outpoint(&self, index: u64) -> H256 {
            BlakeTwo256::hash_of(&(self, index)).into()
        }
    }

    // Transaction output type associated with given Config.
    #[allow(type_alias_bounds)]
    pub type TransactionOutputFor<T: Config> = TransactionOutput<T::AccountId>;

    // Transaction type associated with given Config.
    #[allow(type_alias_bounds)]
    pub type TransactionFor<T: Config> = Transaction<T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn reward_total)]
    pub(super) type RewardTotal<T> = StorageValue<_, Value, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn utxo_store)]
    pub(super) type UtxoStore<T: Config> =
        StorageMap<_, Identity, H256, Option<TransactionOutputFor<T>>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TransactionSuccess(TransactionFor<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block_num: T::BlockNumber) {
            disperse_reward::<T>(&T::authorities(), block_num)
        }
    }

    // Strips a transaction of its Signature fields by replacing value with ZERO-initialized fixed hash.
    pub fn get_simple_transaction<AccountId: Encode + Clone>(
        tx: &Transaction<AccountId>,
    ) -> Vec<u8> {
        let mut trx = tx.clone();
        for input in trx.inputs.iter_mut() {
            input.witness = Vec::new();
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
            let utxo = TransactionOutput::new_pubkey(share_value, *authority);

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
        tx: &TransactionFor<T>,
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
            let input_map: BTreeMap<_, ()> =
                tx.inputs.iter().map(|input| (input.outpoint, ())).collect();
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
                let lock_commitment = input_utxo.destination.lock_commitment();
                ensure!(
                    input.lock_hash() == *lock_commitment,
                    "Lock hash does not match"
                );

                match input_utxo.destination {
                    Destination::Pubkey(pubkey) => {
                        let sig = (&input.witness[..])
                            .try_into()
                            .map_err(|_| "signature length incorrect")?;
                        ensure!(
                            crypto::sr25519_verify(
                                &SR25Sig::from_raw(sig),
                                &simple_tx,
                                &SR25Pub::from_h256(pubkey)
                            ),
                            "signature must be valid"
                        );
                    }
                    Destination::CreatePP(_, _) => {
                        log::info!("TODO validate spending of OP_CREATE");
                    }
                    Destination::CallPP(_, _) => {
                        log::info!("TODO validate spending of OP_CALL");
                    }
                    Destination::ScriptHash(_hash) => {
                        use crate::script::verify;
                        ensure!(
                            verify(&simple_tx, input.witness.clone(), input.lock.clone()).is_ok(),
                            "script verification failed"
                        );
                    }
                    Destination::PubkeyHash(script) => {
                        use crate::script::verify;
                        ensure!(
                            verify(&simple_tx, input.witness.clone(), script).is_ok(),
                            "pubkeyhash verification failed"
                        );
                    }
                }
                total_input =
                    total_input.checked_add(input_utxo.value).ok_or("input value overflow")?;
            } else {
                missing_utxos.push(input.outpoint.clone().as_fixed_bytes().to_vec());
            }
        }

        // Check that outputs are valid
        for (output_index, output) in tx.enumerate_outputs()? {
            // Check the header is valid
            let res = output.validate_header();
            if let Err(e) = res {
                log::error!("Header error: {}", e);
            }
            ensure!(res.is_ok(), "header error. Please check the logs.");

            match output.destination {
                Destination::Pubkey(_)
                | Destination::ScriptHash(_)
                | Destination::PubkeyHash(_) => {
                    ensure!(output.value > 0, "output value must be nonzero");
                    let hash = tx.outpoint(output_index);
                    ensure!(!<UtxoStore<T>>::contains_key(hash), "output already exists");
                    new_utxos.push(hash.as_fixed_bytes().to_vec());
                }
                Destination::CreatePP(_, _) => {
                    log::info!("TODO validate OP_CREATE");
                }
                Destination::CallPP(_, _) => {
                    log::info!("TODO validate OP_CALL");
                }
            }

            // checked add bug in example cod where it uses checked_sub
            total_output = total_output.checked_add(output.value).ok_or("output value overflow")?;
        }

        // if no race condition, check the math
        if missing_utxos.is_empty() {
            ensure!(
                total_input >= total_output,
                "output value must not exceed input value"
            );
            reward = total_input.checked_sub(total_output).ok_or("reward underflow")?;
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
        tx: &TransactionFor<T>,
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

        for (index, output) in tx.enumerate_outputs()? {
            match &output.destination {
                Destination::Pubkey(_)
                | Destination::ScriptHash(_)
                | Destination::PubkeyHash(_) => {
                    let hash = tx.outpoint(index);
                    log::debug!("inserting to UtxoStore {:?} as key {:?}", output, hash);
                    <UtxoStore<T>>::insert(hash, Some(output));
                }
                Destination::CreatePP(script, data) => {
                    create::<T>(caller, script, &data);
                }
                Destination::CallPP(acct_id, data) => {
                    call::<T>(caller, acct_id, data);
                }
            }
        }

        Ok(().into())
    }

    pub fn spend<T: Config>(
        caller: &T::AccountId,
        tx: &TransactionFor<T>,
    ) -> DispatchResultWithPostInfo {
        let tx_validity = validate_transaction::<T>(tx)?;
        ensure!(tx_validity.requires.is_empty(), "missing inputs");
        update_storage::<T>(caller, tx, tx_validity.priority as Value)?;
        Ok(().into())
    }

    /// Pick the UTXOs of `caller` from UtxoStore that satify request `value`
    ///
    /// Return a list of UTXOs that satisfy the request
    /// Return `None` if caller doesn't have enough UTXO
    ///
    // TODO: improve:
    //     - do not return all UTXOs, only enough to satisfy request
    pub fn pick_utxo<T: Config>(
        caller: &T::AccountId,
        _value: Value,
    ) -> (Value, Vec<(Value, H256)>) {
        let mut utxos: Vec<(Value, H256)> = Vec::new();
        let mut total = 0;

        for (hash, utxo) in UtxoStore::<T>::iter() {
            let utxo = utxo.unwrap();

            match utxo.destination {
                Destination::Pubkey(pubkey) => {
                    if caller.encode() == pubkey.encode() {
                        utxos.push((utxo.value, hash));
                        total += utxo.value;
                    }
                }
                _ => {}
            }
        }

        (total, utxos)
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::spend(tx.inputs.len().saturating_add(tx.outputs.len()) as u32))]
        pub fn spend(
            origin: OriginFor<T>,
            tx: Transaction<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            spend::<T>(&ensure_signed(origin)?, &tx)?;
            Self::deposit_event(Event::<T>::TransactionSuccess(tx));
            Ok(().into())
        }

        #[pallet::weight(T::WeightInfo::send_to_pubkey(10_000))]
        pub fn send_to_pubkey(
            origin: OriginFor<T>,
            value: Value,
            destination: H256,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            let (total, utxos) = pick_utxo::<T>(&signer, value);

            ensure!(utxos.len() > 0, "Caller doesn't have enough UTXOs");
            ensure!(total >= value, "Caller doesn't have enough UTXOs");

            let mut inputs: Vec<TransactionInput> = Vec::new();
            for utxo in utxos.iter() {
                inputs.push(TransactionInput::new_empty(utxo.1));
            }

            let pubkey_raw: [u8; 32] = signer.encode().try_into().unwrap();
            let pubkey: Public = Public::from_raw(pubkey_raw);

            let mut tx = Transaction {
                inputs,
                outputs: vec![
                    TransactionOutput::new_pubkey(value, destination),
                    TransactionOutput::new_pubkey(total - value, H256::from(pubkey_raw)),
                ],
            };

            let sig = crypto::sr25519_sign(SR25519, &pubkey, &tx.encode()).unwrap();
            for i in 0..tx.inputs.len() {
                tx.inputs[i].witness = sig.0.to_vec();
            }

            spend::<T>(&signer, &tx)
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub genesis_utxos: Vec<TransactionOutputFor<T>>,
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

impl<T: Config> Pallet<T> {
    pub fn send() -> u32 {
        1337
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
                inputs: vec![TransactionInput::new_with_signature(utxo, sig)],
                outputs: vec![TransactionOutputFor::<T>::new_pubkey(value, address)],
            },
        )
    }
}
