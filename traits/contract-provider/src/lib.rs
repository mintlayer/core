#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::dispatch::Vec;

pub trait ContractProvider {
	fn create(code: &Vec<u8>) -> Result<(), &'static str>;
}
