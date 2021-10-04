#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use codec::{Decode, Encode};
use frame_support::{dispatch::Vec, RuntimeDebug};
use sp_core::{sr25519::Public, H256};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash)]
pub enum TokenInstance {
    Normal {
        id: H256,
        name: Vec<u8>,
        ticker: Vec<u8>,
        supply: u128,
        // We can add another fields like:
        //      pub number_format: NumberFormat,
        //      pub image: UUID,
        //      pub transaction: XXX,
    },
    Nft {
        id: H256,
        data_hash: [u8; 32],
        data_url: Vec<u8>,
        creator_pubkey: Public,
    },
}

impl Default for TokenInstance {
    fn default() -> Self {
        Self::Normal {
            id: H256::zero(),
            name: Vec::new(),
            ticker: Vec::new(),
            supply: 0,
        }
    }
}

impl TokenInstance {
    pub fn new_normal(id: H256, name: Vec<u8>, ticker: Vec<u8>, supply: u128) -> Self {
        Self::Normal {
            id,
            name,
            ticker,
            supply,
        }
    }
    pub fn new_nft(
        id: H256,
        data_hash: [u8; 32],
        data_url: Vec<u8>,
        creator_pubkey: Public,
    ) -> Self {
        Self::Nft {
            id,
            data_hash,
            data_url,
            creator_pubkey,
        }
    }

    pub fn id(&self) -> &H256 {
        match self {
            Self::Normal { id, .. } => id,
            Self::Nft { id, .. } => id,
        }
    }
}

pub type TokenListData = Vec<TokenInstance>;
