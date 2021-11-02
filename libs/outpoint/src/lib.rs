//! Transaction output point.
//!
//! This tiny crate only cointains the outpoint data structure which is just a pair of the previous
//! transaction hash and output index. It is in a separate crate so it could be included in various
//! module withou introducing a massive amount of dependencies.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{H256, RuntimeDebug};

/// Outpoint refers to an output of a transaction by Transaction ID and index.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, RuntimeDebug, Hash, Default)]
pub struct Outpoint {
    pub txid: H256,
    #[codec(compact)] pub index: u32,
}

impl Outpoint {
    pub fn new(txid: H256, index: u32) -> Self {
        Outpoint { txid, index }
    }
}
