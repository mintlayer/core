#![cfg_attr(not(feature = "std"), no_std)]

// use crate::ss58_nostd::*;
// use crate::TransactionOutputFor;
use crate::base58_nostd::{FromBase58, FromBase58Error, ToBase58};
use codec::{Decode, Encode};
// use frame_support::sp_runtime::traits::{BlakeTwo256, Hash};
use frame_support::ensure;
use frame_support::{dispatch::Vec, RuntimeDebug};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use sp_core::crypto::Ss58Codec;
use sp_core::{H160, H256};

const LENGTH_BYTES_TO_REPRESENT_ID: usize = 20;

pub type Value = u128;

pub struct Mlt(pub Value);
impl Mlt {
    pub fn to_munit(&self) -> Value {
        self.0 * 1_000 * 100_000_000
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug)]
enum TokenIdInner {
    // todo: Need to check this
    MLT,
    Asset(H160),
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug)]
pub struct TokenId {
    inner: TokenIdInner,
}

impl TokenId {
    pub fn mlt() -> TokenId {
        TokenId {
            inner: TokenIdInner::MLT,
        }
    }

    pub fn new_asset(first_input_hash: H256) -> TokenId {
        TokenId {
            // We are loosing the first bytes of H256 over here
            inner: TokenIdInner::Asset(H160::from(first_input_hash)),
        }
    }

    pub fn to_string(&self) -> Vec<u8> {
        match self.inner {
            TokenIdInner::MLT => sp_std::vec![],
            TokenIdInner::Asset(hash) => hash.as_bytes().to_base58().to_vec(),
        }
    }

    fn hash160_from_bytes(bytes: &[u8]) -> Result<H160, &'static str> {
        ensure!(
            bytes.len() == LENGTH_BYTES_TO_REPRESENT_ID,
            "Unexpected length of the asset ID"
        );
        let mut buffer = [0u8; 20];
        buffer.copy_from_slice(bytes);
        Ok(H160::from(buffer))
    }

    pub fn from_string(data: &str) -> Result<TokenId, &'static str> {
        let data = data.from_base58().map_err(|x| match x {
            FromBase58Error::InvalidBase58Character { .. } => "Invalid Base58 character",
            FromBase58Error::InvalidBase58Length => "Invalid Base58 length",
        })?;

        let hash = TokenId::hash160_from_bytes(data.as_slice())?;

        Ok(TokenId {
            inner: TokenIdInner::Asset(hash),
        })
    }
}

// We should implement it for Ss58Codec
impl AsMut<[u8]> for TokenId {
    fn as_mut(&mut self) -> &mut [u8] {
        match self.inner {
            TokenIdInner::MLT => &mut [],
            TokenIdInner::Asset(ref mut hash) => hash.as_bytes_mut(),
        }
    }
}

// We should implement it for Ss58Codec
impl AsRef<[u8]> for TokenId {
    fn as_ref(&self) -> &[u8] {
        match self.inner {
            TokenIdInner::MLT => &[],
            TokenIdInner::Asset(ref hash) => hash.as_ref(),
        }
    }
}

// We should implement it for Ss58Codec
impl Default for TokenId {
    fn default() -> Self {
        TokenId::mlt()
    }
}

#[cfg(feature = "std")]
// Unfortunately, the default codec can't be used with std
impl Ss58Codec for TokenId {}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug)]
pub enum OutputData {
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
        metadata_uri: Vec<u8>,
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
        metadata_uri: Vec<u8>,
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

impl OutputData {
    pub(crate) fn id(&self) -> Option<TokenId> {
        match self {
            OutputData::TokenTransferV1 { ref token_id, .. }
            | OutputData::TokenIssuanceV1 { ref token_id, .. }
            | OutputData::NftMintV1 { ref token_id, .. } => Some(token_id.clone()),
            _ => None,
        }
    }
}
