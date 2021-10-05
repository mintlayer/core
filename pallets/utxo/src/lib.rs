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

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod script;
pub mod weights;

pub const SR25519: sp_runtime::KeyTypeId = sp_runtime::KeyTypeId(*b"sr25");

#[frame_support::pallet]
pub mod pallet {
    use crate::SR25519;
    use chainscript::Script;
    use codec::{Decode, Encode};
    use core::convert::TryInto;
    use core::marker::PhantomData;
    use frame_support::weights::PostDispatchInfo;
    use frame_support::{
        dispatch::{DispatchResultWithPostInfo, Vec},
        pallet_prelude::*,
        sp_io::crypto,
        sp_runtime::traits::{BlakeTwo256, Dispatchable, Hash, SaturatedConversion},
        traits::IsSubType,
    };
    use frame_system::pallet_prelude::*;
    use hex_literal::hex;
    use pallet_utxo_tokens::TokenInstance;
    use pp_api::ProgrammablePoolApi;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use sp_core::{
        sp_std::collections::btree_map::BTreeMap,
        sp_std::{str, vec},
        sr25519::{Public as SR25Pub, Signature as SR25Sig},
        H256, H512,
    };
    use sp_runtime::traits::{
        AtLeast32Bit, Zero, /*, StaticLookup , AtLeast32BitUnsigned, Member, One */
    };
    use sp_runtime::DispatchErrorWithPostInfo;

    pub type TokenId = H256;
    pub type Value = u128;
    pub type String = Vec<u8>;

    pub struct Mlt(Value);
    impl Mlt {
        pub fn to_munit(&self) -> Value {
            self.0 * 1_000 * 100_000_000
        }
    }

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
        // Thrown when there is an attempt to mint a duplicate collection.
        NftCollectionExists,
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
        fn token_create(u: u32) -> Weight;
        fn send_to_address(u: u32) -> Weight;
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

    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash)]
    pub enum TxHeaderAndExtraData {
        NormalTx {
            // Normal token ID
            id: TokenId,
        },
        ConfidentialTx, /* not implemented yet */
        Nft {
            id: TokenId,
            data: Vec<u8>,
            data_url: String,
            creator_pkh: H256,
        },
    }

    impl TxHeaderAndExtraData {
        pub fn id(&self) -> Option<TokenId> {
            match self {
                Self::NormalTx { id } => Some(*id),
                Self::Nft { id, .. } => Some(*id),
                _ => None,
            }
        }

        pub fn is_normal(&self) -> bool {
            match self {
                Self::NormalTx { .. } => true,
                _ => false,
            }
        }

        pub fn is_nft(&self) -> bool {
            match self {
                Self::Nft { .. } => true,
                _ => true,
            }
        }
    }

    /// Output of a transaction
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash)]
    pub struct TransactionOutput<AccountId> {
        pub(crate) header: TxHeaderAndExtraData,
        pub(crate) value: Value,
        pub(crate) destination: Destination<AccountId>,
    }

    impl<AccountId> TransactionOutput<AccountId> {
        /// By default the header is 0:
        /// token type for both the value and fee is MLT,
        /// and the signature method is BLS.
        /// functions are available in TXOutputHeaderImpls to update the header.
        pub fn new_pubkey(value: Value, pub_key: H256) -> Self {
            Self {
                header: TxHeaderAndExtraData::NormalTx { id: H256::zero() },
                value,
                destination: Destination::Pubkey(pub_key),
            }
        }

        /// Create a new output to create a smart contract.
        pub fn new_create_pp(value: Value, code: Vec<u8>, data: Vec<u8>) -> Self {
            Self {
                header: TxHeaderAndExtraData::NormalTx { id: H256::zero() },
                value,
                destination: Destination::CreatePP(code, data),
            }
        }

        /// Create a new output to call a smart contract routine.
        pub fn new_call_pp(value: Value, dest_account: AccountId, input: Vec<u8>) -> Self {
            Self {
                header: TxHeaderAndExtraData::NormalTx { id: H256::zero() },
                value,
                destination: Destination::CallPP(dest_account, input),
            }
        }

        pub fn new_token(id: TokenId, value: Value, pub_key: H256) -> Self {
            Self {
                value,
                header: TxHeaderAndExtraData::NormalTx { id },
                destination: Destination::Pubkey(pub_key),
            }
        }

        pub fn new_nft(id: TokenId, data: Vec<u8>, data_url: String, creator: H256) -> Self {
            Self {
                value: 0,
                header: TxHeaderAndExtraData::Nft {
                    id,
                    data,
                    creator_pkh: creator.clone(),
                    data_url,
                },
                destination: Destination::Pubkey(creator),
            }
        }

        /// Create a new output to given script hash.
        pub fn new_script_hash(value: Value, hash: H256) -> Self {
            Self {
                header: TxHeaderAndExtraData::NormalTx { id: H256::zero() },
                value,
                destination: Destination::ScriptHash(hash),
            }
        }

        /// Create a new output to given pubkey hash
        pub fn new_pubkey_hash(value: Value, script: Script) -> Self {
            Self {
                header: TxHeaderAndExtraData::NormalTx { id: H256::zero() },
                value,
                destination: Destination::PubkeyHash(script.into_bytes()),
            }
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
    #[pallet::getter(fn token_list)]
    // todo: Soon it will turn into StorageMap
    pub(super) type TokenList<T> =
        StorageMap<_, Identity, TokenId, Option<TokenInstance>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn reward_total)]
    pub(super) type RewardTotal<T> = StorageValue<_, Value, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn utxo_store)]
    pub(super) type UtxoStore<T: Config> =
        StorageMap<_, Identity, H256, Option<TransactionOutputFor<T>>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        TokenCreated(H256, T::AccountId),
        Minted(H256, T::AccountId, String),
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

        let full_inputs: Vec<(TokenId, TransactionOutputFor<T>)> = tx
            .inputs
            .iter()
            .filter_map(|input| <UtxoStore<T>>::get(&input.outpoint))
            .filter_map(|output| {
                if let TxHeaderAndExtraData::NormalTx { id } = &output.header {
                    Some((*id, output))
                } else {
                    None
                }
            })
            .collect();

        let input_vec: Vec<(TokenId, Value)> =
            full_inputs.iter().map(|output| (output.0, output.1.value)).collect();

        let out_vec: Vec<(TokenId, Value)> = tx
            .outputs
            .iter()
            .filter_map(|output| {
                if let TxHeaderAndExtraData::NormalTx { id } = &output.header {
                    Some((*id, output.value))
                } else {
                    None
                }
            })
            .collect();

        // Check for token creation
        for output in tx.outputs.iter() {
            let tid = match output.header {
                TxHeaderAndExtraData::NormalTx { id } => id,
                _ => continue,
            };
            // If we have input and output for the same token it's not a problem
            if full_inputs.iter().find(|&x| (x.0 == tid) && (x.1 != *output)).is_some() {
                continue;
            } else {
                // But when we don't have an input for token but token id exist in TokenList
                ensure!(
                    !<TokenList<T>>::contains_key(tid),
                    "no inputs for the token id"
                );
            }
        }
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
            } else {
                missing_utxos.push(input.outpoint.clone().as_fixed_bytes().to_vec());
            }
        }

        // Check that outputs are valid
        for (output_index, output) in tx.enumerate_outputs()? {
            match output.destination {
                Destination::Pubkey(_)
                | Destination::ScriptHash(_)
                | Destination::PubkeyHash(_) => {
                    if output.header.is_normal() {
                        ensure!(output.value > 0, "output value must be nonzero");
                    }
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
        }

        // if no race condition, check the math
        if missing_utxos.is_empty() {
            // We have to check sum of input tokens is less or equal to output tokens.
            let mut inputs_sum: BTreeMap<TokenId, Value> = BTreeMap::new();
            let mut outputs_sum: BTreeMap<TokenId, Value> = BTreeMap::new();

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

            let mut new_token_exist = false;
            for output_token in &outputs_sum {
                match inputs_sum.get(&output_token.0) {
                    Some(input_value) => ensure!(
                        input_value >= &output_token.1,
                        "output value must not exceed input value"
                    ),
                    None => {
                        // If the transaction has one an output with a new token ID
                        if new_token_exist {
                            frame_support::fail!("input for the token not found")
                        } else {
                            new_token_exist = true;
                        }
                    }
                }
            }

            // Reward at the moment only in MLT
            reward = if inputs_sum.contains_key(&(H256::zero() as TokenId))
                && outputs_sum.contains_key(&(H256::zero() as TokenId))
            {
                inputs_sum[&(H256::default() as TokenId)]
                    .checked_sub(outputs_sum[&(H256::zero() as TokenId)])
                    .ok_or("reward underflow")?
            } else {
                *inputs_sum.get(&(H256::zero() as TokenId)).ok_or("fee doesn't exist")?
            }
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

    pub fn token_create<T: Config>(
        caller: &T::AccountId,
        public: H256,
        input_for_fee: TransactionInput,
        token_name: String,
        token_ticker: String,
        supply: Value,
    ) -> Result<H256, DispatchErrorWithPostInfo<PostDispatchInfo>> {
        ensure!(token_name.len() <= 25, Error::<T>::Unapproved);
        ensure!(token_ticker.len() <= 5, Error::<T>::Unapproved);
        ensure!(!supply.is_zero(), Error::<T>::MinBalanceZero);

        // Input with MLT FEE
        let fee = UtxoStore::<T>::get(input_for_fee.outpoint).ok_or(Error::<T>::Unapproved)?.value;
        ensure!(fee >= Mlt(100).to_munit(), Error::<T>::Unapproved);

        // Save in UTXO
        let instance = crate::TokenInstance::new_normal(
            BlakeTwo256::hash_of(&(&token_name, &token_ticker)),
            token_name,
            token_ticker,
            supply,
        );
        let token_id = *instance.id();

        ensure!(
            !<TokenList<T>>::contains_key(instance.id()),
            Error::<T>::InUse
        );

        let mut tx = Transaction {
            inputs: crate::vec![
                // Fee an input equal 100 MLT
                input_for_fee,
            ],
            outputs: crate::vec![
                // Output a new tokens
                TransactionOutput::new_token(*instance.id(), supply, public),
            ],
        };

        // We shall make an output to return odd funds
        if fee > Mlt(100).to_munit() {
            tx.outputs.push(TransactionOutput::new_pubkey(
                fee - Mlt(100).to_munit(),
                public,
            ));
        }

        let sig = crypto::sr25519_sign(
            SR25519,
            &sp_core::sr25519::Public::from_h256(public),
            &tx.encode(),
        )
        .ok_or(DispatchError::Token(sp_runtime::TokenError::CannotCreate))?;
        for i in 0..tx.inputs.len() {
            tx.inputs[i].witness = sig.0.to_vec();
        }
        // Success
        spend::<T>(caller, &tx)?;

        // Save in Store
        <TokenList<T>>::insert(token_id, Some(instance));
        Ok(token_id)
    }

    fn mint<T: Config>(
        caller: &T::AccountId,
        creator_pubkey: sp_core::sr25519::Public,
        data_url: String,
        data: Vec<u8>,
    ) -> Result<TokenId, DispatchErrorWithPostInfo<PostDispatchInfo>> {
        let (fee, inputs_hashes) = pick_utxo::<T>(caller, Mlt(100).to_munit());
        ensure!(fee >= Mlt(100).to_munit(), Error::<T>::Unapproved);
        ensure!(data_url.len() <= 50, Error::<T>::Unapproved);

        let instance = TokenInstance::new_nft(
            BlakeTwo256::hash_of(&data),
            data.clone(),
            data_url.clone(),
            creator_pubkey.to_vec(),
        );

        let inputs_for_fee = inputs_hashes
            .iter()
            .filter_map(|x| <UtxoStore<T>>::get(&x))
            .map(|output| TransactionInput::new_empty(BlakeTwo256::hash_of(&(&output, 0 as u64))))
            .collect();

        ensure!(
            !TokenList::<T>::contains_key(instance.id()),
            Error::<T>::NftCollectionExists
        );

        let mut tx = Transaction {
            inputs: inputs_for_fee,
            outputs: crate::vec![
                // Output a new tokens
                TransactionOutput::new_nft(
                    *instance.id(),
                    data,
                    data_url,
                    H256::from(creator_pubkey)
                ),
            ],
        };

        let sig = crypto::sr25519_sign(SR25519, &creator_pubkey, &tx.encode())
            .ok_or(DispatchError::Token(sp_runtime::TokenError::CannotCreate))?;
        for i in 0..tx.inputs.len() {
            tx.inputs[i].witness = sig.0.to_vec();
        }
        // Success
        spend::<T>(caller, &tx)?;

        // Save in Store
        TokenList::<T>::insert(instance.id(), Some(instance.clone()));
        Ok(*instance.id())
    }

    /// Pick the UTXOs of `caller` from UtxoStore that satisfy request `value`
    ///
    /// Return a list of UTXOs that satisfy the request
    /// Return empty vector if caller doesn't have enough UTXO
    ///
    // NOTE: limitation here is that this is only able to pick `Destination::Pubkey`
    // UTXOs because the ownership of those can be easily determined.
    // TODO: keep track of "our" UTXO separately?
    pub fn pick_utxo<T: Config>(caller: &T::AccountId, mut value: Value) -> (Value, Vec<H256>) {
        let mut utxos: Vec<H256> = Vec::new();
        let mut total = 0;

        for (hash, utxo) in UtxoStore::<T>::iter() {
            if let Some(utxo) = utxo {
                match utxo.destination {
                    Destination::Pubkey(pubkey) => {
                        if caller.encode() == pubkey.encode() {
                            utxos.push(hash);
                            total += utxo.value;

                            if utxo.value >= value {
                                break;
                            }
                            value -= utxo.value;
                        }
                    }
                    _ => {}
                }
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

        #[pallet::weight(T::WeightInfo::token_create(768_usize.saturating_add(token_name.len()) as u32))]
        pub fn token_create(
            origin: OriginFor<T>,
            public: H256,
            input_for_fee: TransactionInput,
            token_name: String,
            token_ticker: String,
            supply: Value,
        ) -> DispatchResultWithPostInfo {
            let caller = &ensure_signed(origin)?;
            let token_id = token_create::<T>(
                caller,
                public,
                input_for_fee,
                token_name,
                token_ticker,
                supply,
            )?;
            Self::deposit_event(Event::<T>::TokenCreated(token_id, caller.clone()));
            Ok(().into())
        }

        /// Create a new NFT from the provided NFT info and identify the specified
        /// account as its owner. The ID of the new NFT will be equal to the hash of the info
        /// that defines it, as calculated by the runtime system's hashing algorithm.
        #[pallet::weight(10_000)]
        pub fn mint(
            origin: OriginFor<T>,
            creator_pubkey: sp_core::sr25519::Public,
            data_url: String,
            data: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let caller = &ensure_signed(origin)?;
            let nft_id = mint::<T>(caller, creator_pubkey, data_url.clone(), data)?;
            Self::deposit_event(Event::<T>::Minted(nft_id, caller.clone(), data_url));
            Ok(().into())
        }

        #[pallet::weight(T::WeightInfo::send_to_address(16_u32.saturating_add(address.len() as u32)))]
        pub fn send_to_address(
            origin: OriginFor<T>,
            value: Value,
            address: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            use chainscript::Script;

            ensure!(value > 0, "Value transferred must be larger than zero");
            ensure!(address.len() >= 42, "Invalid Bech32 address");

            let signer = ensure_signed(origin)?;
            let (total, utxos) = pick_utxo::<T>(&signer, value);

            ensure!(total >= value, "Caller doesn't have enough UTXOs");

            let mut inputs: Vec<TransactionInput> = Vec::new();
            for utxo in utxos.iter() {
                inputs.push(TransactionInput::new_empty(*utxo));
            }

            let pubkey_raw: [u8; 32] = match signer.encode().try_into() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to get caller's public key: {:?}", e);
                    return Err("Failed to get caller's public key".into());
                }
            };

            let pubkey: SR25Pub = SR25Pub::from_raw(pubkey_raw);
            let wit_prog = match bech32::wit_prog::WitnessProgram::from_address(
                address[..2].to_vec(),
                address,
            ) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to decode Bech32 address: {:?}", e);
                    return Err("Invalid Bech32 address".into());
                }
            };

            // NOTE: we don't have segwit support yet so use P2PKH
            let script = Script::new_p2pkh(&wit_prog.program);

            let mut tx = Transaction {
                inputs,
                outputs: vec![
                    TransactionOutput::new_pubkey(total - value, H256::from(pubkey_raw)),
                    TransactionOutput::new_pubkey_hash(value, script),
                ],
            };

            let sig = crypto::sr25519_sign(SR25519, &pubkey, &tx.encode())
                .ok_or(DispatchError::Token(sp_runtime::TokenError::CannotCreate))?;
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

use pallet_utxo_tokens::{TokenInstance, TokenListData};

use frame_support::inherent::Vec;
use frame_support::pallet_prelude::DispatchResultWithPostInfo;
use sp_core::{
    crypto::UncheckedFrom,
    {H256, H512},
};
use sp_runtime::sp_std::vec;
use utxo_api::UtxoApi;

impl<T: Config> crate::Pallet<T> {
    pub fn send() -> u32 {
        1337
    }

    pub fn tokens_list() -> TokenListData {
        <TokenList<T>>::iter()
            .enumerate()
            .filter_map(|(_, instance)| instance.1)
            .collect()
    }

    pub fn nft_read(id: H256) -> Option<(/* Data url */ Vec<u8>, /* Data hash */ Vec<u8>)> {
        match TokenList::<T>::get(id)? {
            TokenInstance::Nft { data, data_url, .. } => Some((data_url, data.to_vec())),
            _ => None,
        }
    }
}

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
