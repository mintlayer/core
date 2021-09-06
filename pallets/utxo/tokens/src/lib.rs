#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use codec::{Decode, Encode};
use frame_support::sp_runtime::app_crypto::sp_core::H256;
use frame_support::{
    dispatch::Vec,
    sp_runtime::traits::{BlakeTwo256, Hash},
};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, Hash)] //RuntimeDebug,
pub struct TokenInstance {
    pub id: u64,
    pub name: Vec<u8>,
    pub ticker: Vec<u8>,
    pub supply: u128,
    // pub number_format: NumberFormat,
    // pub image: UUID,
    // pub transaction: XXX,
}

impl Default for TokenInstance {
    fn default() -> Self {
        Self {
            id: 0,
            name: Vec::new(),
            ticker: Vec::new(),
            supply: 0,
        }
    }
}

impl TokenInstance {
    pub fn new(name: Vec<u8>, ticker: Vec<u8>, supply: u128) -> Self {
        Self {
            id: 0, //BlakeTwo256::hash_of(&name).to_low_u64_le(),
            name,
            ticker,
            supply,
        }
    }

    pub fn hash(&self) -> H256 {
        BlakeTwo256::hash_of(&self.name)
    }
}

pub type TokenListData = Vec<TokenInstance>;
