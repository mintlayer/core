// DRY (Don't Repeat Yourself)
#[macro_export]
macro_rules! implement_transaction_verifier {
    () => {
        use crate::sign::TransactionSigMsg;
        use chainscript::sighash::SigHash;

        pub struct TransactionVerifier<'a, T: Config> {
            tx: &'a TransactionFor<T>,
            all_inputs_map: BTreeMap<TokenId, (TransactionInput, TransactionOutputFor<T>)>,
            all_outputs_map: BTreeMap<TokenId, TransactionOutputFor<T>>,
            total_value_of_input_tokens: BTreeMap<TokenId, Value>,
            total_value_of_output_tokens: BTreeMap<TokenId, Value>,
            new_utxos: Vec<Vec<u8>>,
            spended_utxos: Result<
                Vec<TransactionOutput<<T as frame_system::Config>::AccountId>>,
                Vec<Vec<u8>>,
            >,
            reward: u64,
        }

        impl<T: Config> TransactionVerifier<'_, T> {
            // Turn Vector into BTreeMap
            fn init_inputs(
                tx: &TransactionFor<T>,
            ) -> BTreeMap<TokenId, (TransactionInput, TransactionOutputFor<T>)> {
                let input_map: BTreeMap<TokenId, (TransactionInput, TransactionOutputFor<T>)> = tx
                    .inputs
                    .iter()
                    .filter_map(|input| {
                        let token_id =
                            TransactionVerifier::<'_, T>::get_token_id_from_input(input.outpoint)
                                .ok()?;
                        let output =
                            TransactionVerifier::<'_, T>::get_output_by_outpoint(input.outpoint)?;
                        Some((token_id, (input.clone(), output)))
                    })
                    .collect();
                input_map
            }
            // Turn Vector into BTreeMap
            fn init_outputs(tx: &TransactionFor<T>) -> BTreeMap<TokenId, TransactionOutputFor<T>> {
                let output_map: BTreeMap<TokenId, TransactionOutputFor<T>> = tx
                    .outputs
                    .iter()
                    .map(|output| {
                        (
                            TransactionVerifier::<'_, T>::get_token_id_from_output(&output),
                            output.clone(),
                        )
                    })
                    .collect();
                output_map
            }

            fn init_total_value_of_input_tokens(
                all_inputs_map: &BTreeMap<TokenId, (TransactionInput, TransactionOutputFor<T>)>,
            ) -> Result<BTreeMap<TokenId, Value>, &'static str> {
                let mut total_value_of_input_tokens: BTreeMap<TokenId, Value> = BTreeMap::new();
                let mut mlt_amount: Value = 0;
                for (_, (_, (_, input_utxo))) in all_inputs_map.iter().enumerate() {
                    match &input_utxo.data {
                        Some(OutputData::TokenIssuanceV1 {
                            ref token_id,
                            amount_to_issue,
                            ..
                        }) => {
                            // If token has just created we can't meet another amount here.
                            total_value_of_input_tokens.insert(token_id.clone(), *amount_to_issue);
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
                        }
                        Some(OutputData::TokenBurnV1 { .. }) => {
                            // Nothing to do here because tokens no longer exist.
                        }
                        Some(OutputData::NftMintV1 { ref token_id, .. }) => {
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
                total_value_of_input_tokens.insert(TokenId::mlt(), mlt_amount);
                Ok(total_value_of_input_tokens)
            }

            fn init_total_value_of_output_tokens(
                all_outputs_map: &BTreeMap<TokenId, TransactionOutputFor<T>>,
            ) -> Result<BTreeMap<TokenId, Value>, &'static str> {
                let mut total_value_of_output_tokens: BTreeMap<TokenId, Value> = BTreeMap::new();
                let mut mlt_amount: Value = 0;
                for x in all_outputs_map {
                    match &x.1.data {
                        Some(OutputData::TokenIssuanceV1 {
                            ref token_id,
                            amount_to_issue,
                            ..
                        }) => {
                            // If token has just created we can't meet another amount here.
                            total_value_of_output_tokens.insert(token_id.clone(), *amount_to_issue);
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
                                    .ok_or("input value overflow")?,
                            );
                        }
                        Some(OutputData::TokenBurnV1 { .. }) => {
                            // Nothing to do here because tokens no longer exist.
                        }
                        Some(OutputData::NftMintV1 { ref token_id, .. }) => {
                            // If NFT has just created we can't meet another NFT part here.
                            total_value_of_output_tokens.insert(token_id.clone(), 1);
                        }
                        None => {
                            mlt_amount =
                                mlt_amount.checked_add(x.1.value).ok_or("input value overflow")?;
                        }
                    }
                }
                total_value_of_output_tokens.insert(TokenId::mlt(), mlt_amount);
                Ok(total_value_of_output_tokens)
            }

            pub fn new(tx: &TransactionFor<T>) -> Result<TransactionVerifier<T>, &'static str> {
                let all_inputs_map = Self::init_inputs(&tx);
                let all_outputs_map = Self::init_outputs(&tx);
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

            fn get_token_id_from_input(outpoint: H256) -> Result<TokenId, &'static str> {
                //if let Some(input_utxo) = crate::UtxoStore::<T>::get(&outpoint) {
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

                //we want map size and input size to be equal to ensure each is used only once
                ensure!(
                    self.all_inputs_map.len() == self.tx.inputs.len(),
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

                //check each output is defined only once
                ensure!(
                    self.all_outputs_map.len() == self.tx.outputs.len(),
                    "each output should be used once"
                );
                Ok(())
            }

            pub fn checking_signatures(&self) -> Result<(), &'static str> {
                for (index, (_, (input, input_utxo))) in self.all_inputs_map.iter().enumerate() {
                    let spending: Vec<TransactionOutput<T::AccountId>> = self
                        .all_inputs_map
                        .iter()
                        .map(|(_, (_, ref input_utxo))| input_utxo.clone())
                        .collect();
                    match &input_utxo.destination {
                        Destination::Pubkey(pubkey) => {
                            let msg = TransactionSigMsg::construct(
                                SigHash::default(),
                                &self.tx,
                                // todo: Check with Lukas is it correct or no
                                &spending[..],
                                index as u64,
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
                        Destination::CallPP(_, _) => {
                            log::info!("TODO validate spending of OP_CALL");
                        }
                        Destination::ScriptHash(_hash) => {
                            let witness = input.witness.clone();
                            let lock = input.lock.clone();
                            crate::script::verify(
                                &self.tx,
                                // todo: Check with Lukas is it correct or no
                                &spending[..],
                                index as u64,
                                witness,
                                lock,
                            )
                            .map_err(|_| "script verification failed")?;
                        }
                    }
                }

                Ok(())
            }

            pub fn checking_amounts(&self) -> Result<(), &'static str> {
                // if all spent UTXOs are available, check the math and signatures
                let mut new_token_exist = false;
                for (_, (token_id, input_value)) in
                    self.total_value_of_input_tokens.iter().enumerate()
                {
                    match self.total_value_of_output_tokens.get(token_id) {
                        Some(output_value) => ensure!(
                            input_value >= &output_value,
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
                for (output_index, (token_id, output)) in self.all_outputs_map.iter().enumerate() {
                    match output.destination {
                        Destination::Pubkey(_) | Destination::ScriptHash(_) => {
                            if token_id == &TokenId::mlt() {
                                ensure!(output.value > 0, "output value must be nonzero");
                            }
                            let hash = self.tx.outpoint(output_index as u64);
                            ensure!(!<UtxoStore<T>>::contains_key(hash), "output already exists");
                            self.new_utxos.push(hash.as_fixed_bytes().to_vec());
                        }
                        Destination::CreatePP(_, _) => {
                            log::info!("TODO validate OP_CREATE");
                        }
                        Destination::CallPP(_, _) => {
                            log::info!("TODO validate OP_CALL");
                        }
                    }
                }
                Ok(())
            }

            pub fn checking_tokens_transferring(&self) -> Result<(), &'static str> {
                unimplemented!()
            }

            pub fn checking_tokens_issued(&self) -> Result<(), &'static str> {
                unimplemented!()
            }

            pub fn checking_nft_mint(&self) -> Result<(), &'static str> {
                unimplemented!()
            }

            pub fn checking_assets_burn(&self) -> Result<(), &'static str> {
                unimplemented!()
            }

            pub fn calculating_reward(&mut self) -> Result<(), &'static str> {
                use std::convert::TryFrom;
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
