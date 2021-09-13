#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::Vec, weights::Weight};
use sp_core::H256;

pub trait ProgrammablePoolApi {
    type AccountId;

    fn create(
        origin: &Self::AccountId,
        gas: Weight,
        code: &Vec<u8>,
        data: &Vec<u8>,
    ) -> Result<(), &'static str>;

    fn call(
        caller: &Self::AccountId,
        dest: &Self::AccountId,
        gas_limit: Weight,
        utxo_hash: H256,
        utxo_value: u128,
        input_data: &Vec<u8>,
    ) -> Result<(), &'static str>;
}
