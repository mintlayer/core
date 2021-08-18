#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::Vec, weights::Weight};

pub trait ContractProvider {
    type AccountId;

	fn create(
        origin: &Self::AccountId,
        gas: Weight,
        code: &Vec<u8>
    ) -> Result<(), &'static str>;

    fn call(
        caller: &Self::AccountId,
        dest: &Self::AccountId,
		gas_limit: Weight,
		input_data: &Vec<u8>,
    ) -> Result<(), &'static str>;
}
