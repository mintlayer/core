#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use codec::{Decode, Encode};
use frame_support::{dispatch::Vec, RuntimeDebug};
use sp_core::{sr25519::Public, H256};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash)]
pub struct NftDataRaw {
    inner: Vec<u8>,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash)]
pub enum NftData {
    Hash32([u8; 32]),
    Hash64([u8; 64]),
    Raw(Vec<u8>),
    // Or any type that you want to implement
}

impl NftDataRaw {
    pub fn new(data: Vec<u8>) -> NftDataRaw {
        Self { inner: data }
    }

    pub fn into_data(&mut self) -> Option<NftData> {
        NftData::decode(&mut self.as_slice()).ok()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.inner.clone()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.inner.as_slice()
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash)]
pub struct NftOwnerRaw {
    inner: Vec<u8>,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub enum NftOwner {
    Sr25519(Public),
    Raw(Vec<u8>),
    // Or any type that you want to implement
}

impl NftOwnerRaw {
    pub fn new(data: Vec<u8>) -> Self {
        Self { inner: data }
    }

    pub fn into_data(&mut self) -> Option<NftOwner> {
        NftOwner::decode(&mut self.as_slice()).ok()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.inner.clone()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.inner.as_slice()
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Hash)]
pub enum TokenInstance {
    Normal {
        id: H256,
        version: u16,
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
        version: u16,
        data: NftDataRaw,
        data_url: Vec<u8>,
        creator_pubkey: NftOwnerRaw,
    },
}

impl Default for TokenInstance {
    fn default() -> Self {
        Self::Normal {
            id: H256::zero(),
            version: 0,
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
            version: 0,
            name,
            ticker,
            supply,
        }
    }
    pub fn new_nft(id: H256, data: Vec<u8>, data_url: Vec<u8>, creator_pubkey: Vec<u8>) -> Self {
        Self::Nft {
            id,
            version: 0,
            data: NftDataRaw::new(data),
            data_url,
            creator_pubkey: NftOwnerRaw::new(creator_pubkey),
        }
    }

    pub fn id(&self) -> &H256 {
        match self {
            Self::Normal { id, .. } => id,
            Self::Nft { id, .. } => id,
        }
    }

    pub fn version(&self) -> u16 {
        *match self {
            Self::Normal { version, .. } => version,
            Self::Nft { version, .. } => version,
        }
    }
}

pub type TokenListData = Vec<TokenInstance>;
