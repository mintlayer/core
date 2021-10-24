use crate::tokens::{OutputData, TokenId};
use crate::{/*Transaction,*/ TransactionFor, TransactionOutputFor};
use frame_support::ensure;
use frame_support::pallet_prelude::ValidTransaction;
use sp_core::sp_std::collections::btree_map::BTreeMap;
use sp_core::H256;

pub struct TransactionVerifier<'a, T: frame_system::Config> {
    tx: &'a TransactionFor<T>,
    input_map: Option<BTreeMap<TokenId, TransactionOutputFor<T>>>,
    output_map: Option<BTreeMap<TokenId, TransactionOutputFor<T>>>,
}

impl<T: frame_system::Config> TransactionVerifier<'_, T> {
    pub fn new(tx: &TransactionFor<T>) -> TransactionVerifier<T> {
        TransactionVerifier {
            tx,
            input_map: None,
            output_map: None,
        }
    }

    fn get_token_id_from_input(_outpoint: H256) -> TokenId {
        unimplemented!()
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

    fn get_output_by_outpoint(_outpoint: H256) -> TransactionOutputFor<T> {
        unimplemented!()
    }

    pub fn checking_inputs(&mut self) -> Result<TransactionVerifier<T>, &'static str> {
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

        let input_map: BTreeMap<TokenId, TransactionOutputFor<T>> = self
            .tx
            .inputs
            .iter()
            .map(|input| {
                (
                    TransactionVerifier::<'_, T>::get_token_id_from_input(input.outpoint),
                    TransactionVerifier::<'_, T>::get_output_by_outpoint(input.outpoint),
                )
            })
            .collect();
        //we want map size and input size to be equal to ensure each is used only once
        ensure!(
            input_map.len() == self.tx.inputs.len(),
            "each input should be used only once"
        );
        self.input_map = Some(input_map);
        unimplemented!()
    }

    pub fn checking_outputs(&mut self) -> Result<TransactionVerifier<T>, &'static str> {
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

        let output_map: BTreeMap<TokenId, TransactionOutputFor<T>> = self
            .tx
            .outputs
            .iter()
            .map(|output| {
                (
                    TransactionVerifier::<'_, T>::get_token_id_from_output(&output),
                    output.clone(),
                )
            })
            .collect();
        //check each output is defined only once
        ensure!(
            output_map.len() == self.tx.outputs.len(),
            "each output should be used once"
        );
        self.output_map = Some(output_map);
        unimplemented!()
    }

    pub fn checking_signatures(&self) -> Result<TransactionVerifier<T>, &'static str> {
        unimplemented!()
    }

    pub fn checking_utxos_exists(&self) -> Result<TransactionVerifier<T>, &'static str> {
        unimplemented!()
    }

    pub fn checking_tokens_transferring(&self) -> Result<TransactionVerifier<T>, &'static str> {
        unimplemented!()
    }

    pub fn checking_tokens_issued(&self) -> Result<TransactionVerifier<T>, &'static str> {
        unimplemented!()
    }

    pub fn checking_nft_mint(&self) -> Result<TransactionVerifier<T>, &'static str> {
        unimplemented!()
    }

    pub fn checking_assets_burn(&self) -> Result<TransactionVerifier<T>, &'static str> {
        unimplemented!()
    }

    pub fn calculating_reward(&self) -> Result<TransactionVerifier<T>, &'static str> {
        unimplemented!()
    }

    pub fn collect_result(&self) -> Result<ValidTransaction, &'static str> {
        unimplemented!()
    }
}
