#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{dispatch::Vec, RuntimeDebug};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{sr25519::Public, H256};

pub type Value = u128;

pub struct Mlt(Value);
impl Mlt {
    pub fn to_munit(&self) -> Value {
        self.0 * 1_000 * 100_000_000
    }
}

pub type TokenId = H256;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug)]
pub enum TxData {
    // TokenTransfer data to another user. If it is a token, then the token data must also be transferred to the recipient.
    #[codec(index = 1)]
    TokenTransferV1 { token_id: TokenId, amount: u128 },
    // A new token creation
    #[codec(index = 2)]
    TokenIssuanceV1 {
        token_id: TokenId,
        token_ticker: Vec<u8>,
        amount_to_issue: u128,
        // Should be not more than 18 numbers
        number_of_decimals: u8,
        metadata_URI: Vec<u8>,
    },
    // Burning a token or NFT
    #[codec(index = 3)]
    TokenBurnV1 {
        token_id: TokenId,
        amount_to_burn: u128,
    },
    // A new NFT creation
    #[codec(index = 4)]
    NftMintV1 {
        token_id: TokenId,
        data_hash: NftDataHash,
        metadata_URI: Vec<u8>,
    },
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug)]
pub enum NftDataHash {
    #[codec(index = 1)]
    Hash32([u8; 32]),
    #[codec(index = 2)]
    Raw(Vec<u8>),
    // Or any type that you want to implement
}
