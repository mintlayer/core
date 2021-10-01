#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use codec::{Decode, Encode};
use frame_support::{dispatch::Vec, RuntimeDebug};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash)]
pub struct TokenInstance {
    pub id: u64,
    pub name: Vec<u8>,
    pub ticker: Vec<u8>,
    pub supply: u128,
    // We can add another fields like:
    //      pub number_format: NumberFormat,
    //      pub image: UUID,
    //      pub transaction: XXX,
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
    pub fn new(id: u64, name: Vec<u8>, ticker: Vec<u8>, supply: u128) -> Self {
        Self {
            id,
            name,
            ticker,
            supply,
        }
    }
}

pub type TokenListData = Vec<TokenInstance>;
