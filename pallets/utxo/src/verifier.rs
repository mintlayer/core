#[macro_export]
// The Substrate has a big macros ecosystem. That could be easily broken if T:Config will using in
// other mod instead of lib.rs. Due to we have not enough time for quality decomposition lib.rs to
// I decide to move this part of the code in the macro.
//
// At the moment, this piece of code is rough. After the test-net, we will return to https://github.com/mintlayer/core/issues/81
// and decide how it make it better. The main problem is that there are a lot of cycles. We should split into
// stages and use all of these checks as an array of functions that we will call on a couple of main cycles.
// But, at the moment it works and it is suitable for the test-net.

macro_rules! implement_transaction_verifier {
    () => {
        use crate::sign::TransactionSigMsg;
        use chainscript::sighash::SigHash;

        // The main object, where stored temporary data about a tx
        pub struct TransactionVerifier<'a, T: Config> {
            // Pointer to the transaction
            tx: &'a TransactionFor<T>,
            // Vec of inputs for each Token ID
            all_inputs_map: BTreeMap<TokenId, Vec<(TransactionInput, TransactionOutputFor<T>)>>,
            // Vec of outputs for each Token ID
            all_outputs_map: BTreeMap<TokenId, Vec<TransactionOutputFor<T>>>,
            // The total summary value of the tokens in inputs for each TokenID
            total_value_of_input_tokens: BTreeMap<TokenId, Value>,
            // The total summary value of the tokens in outputs for each TokenID
            total_value_of_output_tokens: BTreeMap<TokenId, Value>,
            // Vec of outputs that should be written
            new_utxos: Vec<Vec<u8>>,
            // For more information have a look at checking_utxos_exists
            spended_utxos: Result<
                Vec<TransactionOutput<<T as frame_system::Config>::AccountId>>,
                Vec<Vec<u8>>,
            >,
            // The total reward for this tx
            reward: u64,
        }

        impl<T: Config> TransactionVerifier<'_, T> {
            pub fn new(tx: &TransactionFor<T>) -> Result<TransactionVerifier<T>, &'static str> {
                // Verify absolute time lock
                ensure!(
                    tx.check_time_lock::<T>(),
                    "Time lock restrictions not satisfied"
                );
                // Init
                let all_inputs_map = Self::init_inputs(&tx)?;
                let all_outputs_map = Self::init_outputs(&tx)?;
                let total_value_of_input_tokens =
                    Self::init_total_value_of_input_tokens(&all_inputs_map)?;
                let total_value_of_output_tokens =
                    Self::init_total_value_of_output_tokens(&all_outputs_map)?;
                Ok(TransactionVerifier {
                    tx,
                    all_inputs_map,
                    all_outputs_map,
                    total_value_of_input_tokens,
                    total_value_of_output_tokens,
                    new_utxos: Vec::new(),
                    spended_utxos: Ok(Vec::new()),
                    reward: 0,
                })
            }

            // Turn Vector into BTreeMap
            fn init_inputs(
                tx: &TransactionFor<T>,
            ) -> Result<
                BTreeMap<TokenId, Vec<(TransactionInput, TransactionOutputFor<T>)>>,
                &'static str,
            > {
                let mut input_map: BTreeMap<
                    TokenId,
                    Vec<(TransactionInput, TransactionOutputFor<T>)>,
                > = BTreeMap::new();

                for input in &tx.inputs {
                    let token_id =
                        TransactionVerifier::<'_, T>::get_token_id_from_input(input.outpoint)?;
                    let output =
                        TransactionVerifier::<'_, T>::get_output_by_outpoint(input.outpoint)
                            .ok_or("missing inputs")?;

                    if let Some(inputs) = input_map.get_mut(&token_id) {
                        inputs.push((input.clone(), output));
                    } else {
                        input_map.insert(token_id, vec![(input.clone(), output)]);
                    }
                }
                Ok(input_map)
            }
            // Turn Vector into BTreeMap
            fn init_outputs(
                tx: &TransactionFor<T>,
            ) -> Result<BTreeMap<TokenId, Vec<TransactionOutputFor<T>>>, &'static str> {
                let mut count = 0;
                let mut output_map: BTreeMap<TokenId, Vec<TransactionOutputFor<T>>> =
                    BTreeMap::new();

                for output in &tx.outputs {
                    let token_id = TransactionVerifier::<'_, T>::get_token_id_from_output(&output);
                    if let Some(outputs) = output_map.get_mut(&token_id) {
                        count += 1;
                        outputs.push(output.clone());
                    } else {
                        count += 1;
                        output_map.insert(token_id, vec![output.clone()]);
                    }
                }
                ensure!(count == tx.outputs.len(), "can't load all outputs");
                Ok(output_map)
            }

            fn init_total_value_of_input_tokens(
                all_inputs_map: &BTreeMap<
                    TokenId,
                    Vec<(TransactionInput, TransactionOutputFor<T>)>,
                >,
            ) -> Result<BTreeMap<TokenId, Value>, &'static str> {
                let mut total_value_of_input_tokens: BTreeMap<TokenId, Value> = BTreeMap::new();
                let mut mlt_amount: Value = 0;
                for (_, (_, input_vec)) in all_inputs_map.iter().enumerate() {
                    for (_, input_utxo) in input_vec {
                        match &input_utxo.data {
                            Some(OutputData::TokenIssuanceV1 {
                                ref token_id,
                                token_ticker,
                                amount_to_issue,
                                number_of_decimals,
                                metadata_uri,
                            }) => {
                                // We have to check is this token already issued?
                                ensure!(
                                    PointerToIssueToken::<T>::contains_key(token_id),
                                    "token has never been issued"
                                );
                                ensure!(
                                    token_id != &TokenId::mlt(),
                                    "unable to use mlt as a token id"
                                );
                                ensure!(
                                    token_ticker.is_ascii(),
                                    "token ticker has none ascii characters"
                                );
                                ensure!(
                                    metadata_uri.is_ascii(),
                                    "metadata uri has none ascii characters"
                                );
                                ensure!(token_ticker.len() <= 5, "token ticker is too long");
                                ensure!(!token_ticker.is_empty(), "token ticker can't be empty");
                                ensure!(
                                    metadata_uri.len() <= 100,
                                    "token metadata uri is too long"
                                );
                                ensure!(amount_to_issue > &0u128, "output value must be nonzero");
                                ensure!(number_of_decimals <= &18, "too long decimals");
                                // If token has just created we can't meet another amount here.
                                total_value_of_input_tokens
                                    .insert(token_id.clone(), *amount_to_issue);
                                // But probably in this input we have a fee
                                mlt_amount = mlt_amount
                                    .checked_add(input_utxo.value)
                                    .ok_or("input value overflow")?;
                            }
                            Some(OutputData::TokenTransferV1 {
                                ref token_id,
                                amount,
                                ..
                            }) => {
                                total_value_of_input_tokens.insert(
                                    token_id.clone(),
                                    total_value_of_input_tokens
                                        .get(token_id)
                                        .unwrap_or(&0)
                                        .checked_add(*amount)
                                        .ok_or("input value overflow")?,
                                );
                                // But probably in this input we have a fee
                                mlt_amount = mlt_amount
                                    .checked_add(input_utxo.value)
                                    .ok_or("input value overflow")?;
                            }
                            Some(OutputData::TokenBurnV1 { .. }) => {
                                // Nothing to do here because tokens no longer exist.
                            }
                            Some(OutputData::NftMintV1 {
                                ref token_id,
                                data_hash,
                                metadata_uri,
                            }) => {
                                // We have to check is this token already issued?
                                ensure!(
                                    PointerToIssueToken::<T>::contains_key(token_id),
                                    "unable to use an input where NFT has not minted yet"
                                );

                                // Check is this digital data unique?
                                ensure!(
                                    NftUniqueDataHash::<T>::contains_key(data_hash),
                                    "unable to use an input where NFT digital data was changed"
                                );

                                ensure!(
                                    token_id != &TokenId::mlt(),
                                    "unable to use mlt as a token id"
                                );
                                ensure!(
                                    metadata_uri.is_ascii(),
                                    "metadata uri has none ascii characters"
                                );
                                // If NFT has just created we can't meet another NFT part here.
                                total_value_of_input_tokens.insert(token_id.clone(), 1);
                            }
                            None => {
                                mlt_amount = mlt_amount
                                    .checked_add(input_utxo.value)
                                    .ok_or("input value overflow")?;
                            }
                        }
                    }
                }
                total_value_of_input_tokens.insert(TokenId::mlt(), mlt_amount);
                Ok(total_value_of_input_tokens)
            }

            fn init_total_value_of_output_tokens(
                all_outputs_map: &BTreeMap<TokenId, Vec<TransactionOutputFor<T>>>,
            ) -> Result<BTreeMap<TokenId, Value>, &'static str> {
                let mut total_value_of_output_tokens: BTreeMap<TokenId, Value> = BTreeMap::new();
                let mut mlt_amount: Value = 0;
                for (_, (_, outputs_vec)) in all_outputs_map.iter().enumerate() {
                    for utxo in outputs_vec {
                        // for x in all_outputs_map {
                        match &utxo.data {
                            Some(OutputData::TokenIssuanceV1 {
                                ref token_id,
                                token_ticker,
                                amount_to_issue,
                                number_of_decimals,
                                metadata_uri,
                            }) => {
                                // We have to check is this token already issued?
                                ensure!(
                                    !PointerToIssueToken::<T>::contains_key(token_id),
                                    "token has already been issued"
                                );
                                ensure!(
                                    token_id != &TokenId::mlt(),
                                    "unable to use mlt as a token id"
                                );
                                ensure!(
                                    token_ticker.is_ascii(),
                                    "token ticker has none ascii characters"
                                );
                                ensure!(
                                    metadata_uri.is_ascii(),
                                    "metadata uri has none ascii characters"
                                );
                                ensure!(token_ticker.len() <= 5, "token ticker is too long");
                                ensure!(!token_ticker.is_empty(), "token ticker can't be empty");
                                ensure!(
                                    metadata_uri.len() <= 100,
                                    "token metadata uri is too long"
                                );
                                ensure!(amount_to_issue > &0u128, "output value must be nonzero");
                                ensure!(number_of_decimals <= &18, "too long decimals");

                                // If token has just created we can't meet another amount here.
                                total_value_of_output_tokens
                                    .insert(token_id.clone(), *amount_to_issue);
                                // But probably in this input we have a fee
                                mlt_amount = mlt_amount
                                    .checked_add(utxo.value)
                                    .ok_or("input value overflow")?;
                            }
                            Some(OutputData::TokenTransferV1 {
                                ref token_id,
                                amount,
                                ..
                            }) => {
                                total_value_of_output_tokens.insert(
                                    token_id.clone(),
                                    total_value_of_output_tokens
                                        .get(token_id)
                                        .unwrap_or(&0)
                                        .checked_add(*amount)
                                        .ok_or("output value overflow")?,
                                );
                                // But probably in this input we have a fee
                                mlt_amount = mlt_amount
                                    .checked_add(utxo.value)
                                    .ok_or("input value overflow")?;
                            }
                            Some(OutputData::TokenBurnV1 { .. }) => {
                                // Nothing to do here because tokens no longer exist.
                            }
                            Some(OutputData::NftMintV1 {
                                ref token_id,
                                data_hash,
                                metadata_uri,
                            }) => {
                                // We have to check is this token already issued?
                                ensure!(
                                    !PointerToIssueToken::<T>::contains_key(token_id),
                                    "token has already been issued"
                                );

                                // Check is this digital data unique?
                                ensure!(
                                    !<NftUniqueDataHash<T>>::contains_key(data_hash),
                                    "digital data has already been minted"
                                );

                                ensure!(
                                    token_id != &TokenId::mlt(),
                                    "unable to use mlt as a token id"
                                );
                                ensure!(
                                    metadata_uri.is_ascii(),
                                    "metadata uri has none ascii characters"
                                );
                                // If NFT has just created we can't meet another NFT part here.
                                total_value_of_output_tokens.insert(token_id.clone(), 1);
                            }
                            None => {
                                mlt_amount = mlt_amount
                                    .checked_add(utxo.value)
                                    .ok_or("output value overflow")?;
                            }
                        }
                    }
                }
                total_value_of_output_tokens.insert(TokenId::mlt(), mlt_amount);
                Ok(total_value_of_output_tokens)
            }

            fn get_token_id_from_input(outpoint: H256) -> Result<TokenId, &'static str> {
                if let Some(input_utxo) = <UtxoStore<T>>::get(outpoint) {
                    match input_utxo.data {
                        Some(data) => data.id().ok_or("Token had burned or input incorrect"),
                        None => Ok(TokenId::mlt()),
                    }
                } else {
                    Ok(TokenId::mlt())
                }
            }

            fn get_token_id_from_output(output: &TransactionOutputFor<T>) -> TokenId {
                match output.data {
                    Some(OutputData::TokenTransferV1 { ref token_id, .. })
                    | Some(OutputData::TokenIssuanceV1 { ref token_id, .. })
                    | Some(OutputData::NftMintV1 { ref token_id, .. }) => token_id.clone(),
                    Some(OutputData::TokenBurnV1 { .. }) => unreachable!(),
                    _ => TokenId::mlt(),
                }
            }

            fn get_output_by_outpoint(outpoint: H256) -> Option<TransactionOutputFor<T>> {
                <UtxoStore<T>>::get(outpoint)
            }

            pub fn checking_inputs(&mut self) -> Result<(), &'static str> {
                //ensure rather than assert to avoid panic
                //both inputs and outputs should contain at least 1 and at most u32::MAX - 1 entries
                ensure!(!self.tx.inputs.is_empty(), "no inputs");
                ensure!(
                    self.tx.inputs.len() < (u32::MAX as usize),
                    "too many inputs"
                );

                //ensure each input is used only a single time
                //maps each input into btree
                //if map.len() > num of inputs then fail
                //https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
                //WARNING workshop code has a bug here
                //https://github.com/substrate-developer-hub/utxo-workshop/blob/workshop/runtime/src/utxo.rs
                //input_map.len() > transaction.inputs.len() //THIS IS WRONG
                let input_map: BTreeMap<_, ()> =
                    self.tx.inputs.iter().map(|input| (input.outpoint, ())).collect();
                //we want map size and input size to be equal to ensure each is used only once
                ensure!(
                    input_map.len() == self.tx.inputs.len(),
                    "each input should be used only once"
                );
                Ok(())
            }

            pub fn checking_outputs(&mut self) -> Result<(), &'static str> {
                //ensure rather than assert to avoid panic
                //both inputs and outputs should contain at least 1 and at most u32::MAX - 1 entries
                ensure!(!self.tx.outputs.is_empty(), "no outputs");
                ensure!(
                    self.tx.outputs.len() < (u32::MAX as usize),
                    "too many outputs"
                );

                //ensure each output is unique
                //map each output to btree to count unique elements
                //WARNING example code has a bug here
                //out_map.len() != transaction.outputs.len() //THIS IS WRONG
                let out_map: BTreeMap<_, ()> =
                    self.tx.outputs.iter().map(|output| (output, ())).collect();

                //check each output is defined only once
                ensure!(
                    out_map.len() == self.tx.outputs.len(),
                    "each output should be used once"
                );
                Ok(())
            }

            pub fn checking_signatures(&self) -> Result<(), &'static str> {
                for (index, (_, inputs_vec)) in self.all_inputs_map.iter().enumerate() {
                    for (sub_index, (input, input_utxo)) in inputs_vec.iter().enumerate() {
                        let spending_utxos: Vec<TransactionOutput<T::AccountId>> = self
                            .all_inputs_map
                            .iter()
                            .map(|(_, inputs_vec)| {
                                inputs_vec
                                    .iter()
                                    .map(|item| item.1.clone())
                                    .collect::<Vec<TransactionOutput<T::AccountId>>>()
                            })
                            .flatten()
                            .collect();
                        match &input_utxo.destination {
                            Destination::Pubkey(pubkey) => {
                                let msg = TransactionSigMsg::construct(
                                    SigHash::default(),
                                    &self.tx,
                                    &spending_utxos[..],
                                    (index + sub_index) as u64,
                                    u32::MAX,
                                );
                                let ok = crate::sign::Public::Schnorr(*pubkey)
                                    .parse_sig(&input.witness[..])
                                    .ok_or("bad signature format")?
                                    .verify(&msg);
                                ensure!(ok, "signature must be valid");
                            }
                            Destination::CreatePP(_, _) => {
                                log::info!("TODO validate spending of OP_CREATE");
                            }
                            Destination::CallPP(_, _, _) => {
                                let spend = u16::from_le_bytes(
                                    input.witness[1..].try_into().or_else(|_| {
                                        Err(DispatchError::Other(
                                            "Failed to convert witness to an opcode",
                                        ))
                                    })?,
                                );
                                ensure!(spend == 0x1337, "OP_SPEND not found");
                            }
                            Destination::ScriptHash(_hash) => {
                                let witness = input.witness.clone();
                                let lock = input.lock.clone();
                                crate::script::verify(
                                    &self.tx,
                                    // todo: Check with Lukas is it correct or no
                                    &spending_utxos[..],
                                    (index + sub_index) as u64,
                                    witness,
                                    lock,
                                )
                                .map_err(|_| "script verification failed")?;
                            }
                        }
                    }
                }

                Ok(())
            }

            pub fn checking_amounts(&self) -> Result<(), &'static str> {
                let mut num_creations = 0;
                for (_, (token_id, output_value)) in
                    self.total_value_of_output_tokens.iter().enumerate()
                {
                    match self.total_value_of_input_tokens.get(token_id) {
                        Some(input_value) => ensure!(
                            input_value >= &output_value,
                            "output value must not exceed input value"
                        ),
                        None => {
                            match self.all_outputs_map.get(token_id) {
                                Some(outputs_vec) => {
                                    // We have not any input for this token, perhaps it's token creation
                                    ensure!(
                                        outputs_vec.len() == 1,
                                        "attempting double creation token failed"
                                    );
                                    match outputs_vec[0].data {
                                        None
                                        | Some(OutputData::TokenTransferV1 { .. })
                                        | Some(OutputData::TokenBurnV1 { .. }) => {
                                            frame_support::fail!("input for the token not found")
                                        }
                                        Some(OutputData::NftMintV1 { .. })
                                        | Some(OutputData::TokenIssuanceV1 { .. }) => {
                                            num_creations += 1;
                                            continue;
                                        }
                                    }
                                }
                                None => unreachable!(),
                            }
                        }
                    }
                }
                // Check that enough fee
                let mlt = self
                    .total_value_of_input_tokens
                    .get(&TokenId::mlt())
                    .ok_or("not found MLT fees")?;
                if cfg!(test) {
                    // For tests we will use a small amount of MLT
                    ensure!(mlt >= &(num_creations * 10), "insufficient fee");
                } else {
                    // If we are not in tests, we should use 100 MLT for each token creation
                    ensure!(
                        mlt >= &(num_creations * crate::tokens::Mlt(100).to_munit()),
                        "insufficient fee"
                    )
                }
                Ok(())
            }

            pub fn checking_utxos_exists(&mut self) -> Result<(), &'static str> {
                // Resolve the transaction inputs by looking up UTXOs being spent by them.
                //
                // This will contain one of the following:
                // * Ok(utxos): a vector of UTXOs each input spends.
                // * Err(missing): a vector of outputs missing from the store

                self.spended_utxos = {
                    let mut missing = Vec::new();
                    let mut resolved: Vec<TransactionOutputFor<T>> = Vec::new();

                    for input in &self.tx.inputs {
                        if let Some(input_utxo) = <UtxoStore<T>>::get(&input.outpoint) {
                            let lock_commitment = input_utxo.destination.lock_commitment();
                            ensure!(
                                input.lock_hash() == *lock_commitment,
                                "Lock hash does not match"
                            );
                            resolved.push(input_utxo);
                        } else {
                            missing.push(input.outpoint.clone().as_fixed_bytes().to_vec());
                        }
                    }
                    missing.is_empty().then(|| resolved).ok_or(missing)
                };

                // Check that outputs are valid
                for (output_index, (token_id, outputs_vec)) in
                    self.all_outputs_map.iter().enumerate()
                {
                    for (sub_index, output) in outputs_vec.iter().enumerate() {
                        let hash = self.tx.outpoint((output_index + sub_index) as u64);
                        ensure!(!<UtxoStore<T>>::contains_key(hash), "output already exists");
                        if token_id == &TokenId::mlt() {
                            ensure!(output.value > 0, "output value must be nonzero");
                        }
                        self.new_utxos.push(hash.as_fixed_bytes().to_vec());
                    }
                }
                Ok(())
            }

            pub fn calculating_reward(&mut self) -> Result<(), &'static str> {
                use sp_std::convert::TryFrom;
                // Reward at the moment only in MLT
                self.reward = if self.total_value_of_input_tokens.contains_key(&TokenId::mlt())
                    && self.total_value_of_output_tokens.contains_key(&(TokenId::mlt()))
                {
                    u64::try_from(
                        self.total_value_of_input_tokens[&TokenId::mlt()]
                            .checked_sub(self.total_value_of_output_tokens[&TokenId::mlt()])
                            .ok_or("reward underflow")?,
                    )
                    .map_err(|_e| "too big amount of fee")?
                } else {
                    u64::try_from(
                        *self
                            .total_value_of_input_tokens
                            .get(&TokenId::mlt())
                            .ok_or("fee doesn't exist")?,
                    )
                    .map_err(|_e| "too big amount of fee")?
                };
                Ok(())
            }

            pub fn collect_result(&self) -> Result<ValidTransaction, &'static str> {
                Ok(ValidTransaction {
                    priority: self.reward,
                    requires: self.spended_utxos.clone().map_or_else(|x| x, |_| Vec::new()),
                    provides: self.new_utxos.clone(),
                    longevity: TransactionLongevity::MAX,
                    propagate: true,
                })
            }
        }
    };
}
