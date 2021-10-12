// Copyright (c) 2021 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://spdx.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Author(s): L. Kuklinek

//! Verify transaction signatures
//!
//! This module provides two bits of functionality:
//! 1. Tools to construct byte string to be signed when signing a transaction.
//!    See [TransactionOutputSigMsg::construct].
//! 2. Tools to verify signatures using multiple signature schemes.
//!    See [Public] and [SignatureData].

use crate::{Transaction, TransactionOutput};

use chainscript::context::ParseResult;
pub use chainscript::sighash::SigHash;
use chainscript::sighash::{InputMode, OutputMode};
use codec::{Decode, DecodeAll, Encode};
use frame_support::sp_io::crypto;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, H256};
use sp_runtime::traits::{BlakeTwo256, Hash};
use sp_std::prelude::*;
use variant_count::VariantCount;

/// Transaction data comitted to in a signature.
#[derive(Eq, PartialEq, Clone, Encode)]
pub struct TransactionSigMsg {
    /// Sighash
    sighash: SigHash,
    /// Information about inputs
    inputs: TransactionInputSigMsg,
    /// Information about outputs
    outputs: TransactionOutputSigMsg,
    /// OP_CODESEPARATOR position (or 0xffffffff if none seen so far)
    codesep_idx: u32,
}

/// Transaction input data comitted to in a signature.
#[derive(Eq, PartialEq, Clone, Encode)]
enum TransactionInputSigMsg {
    /// Commit to all inputs
    CommitWhoPays {
        outpoints: H256,
        spending: H256,
        index: u64,
    },
    /// Commit to this input only
    AnyoneCanPay { outpoint: H256, spending: H256 },
}

/// Transaction output data comitted to in a signature.
#[derive(Eq, PartialEq, Clone, Encode)]
enum TransactionOutputSigMsg {
    /// Commit to all transaction outputs
    #[codec(index = 0x01)]
    All { outputs: H256 },

    /// Don't commit to any outputs (i.e. outputs can be changed freely)
    #[codec(index = 0x02)]
    None,

    /// Commit to just one output that corresponds to the current input
    #[codec(index = 0x03)]
    Single { output: H256 },
}

impl TransactionSigMsg {
    /// Create a `TransactionSigMsg` from a transaction, spent outputs, current index
    /// and other context information according to given sighash.
    ///
    /// The `sighash` parameter specifies which parts of the transaction `tx` are signed. A list of
    /// UTXOs corresponding to transaction inputs being spent is also required and has to be passed
    /// in the `spending` parameter. No validation is done to ensure the UTXOs really match the
    /// inputs, it is resposnsibility of the caller to verify it. The input being sign is passed in
    /// the `index` argument and the index of the last `OP_CODESEPARATOR` is in `codesep_idx`.
    ///
    /// TODO This could be improved by pre-calculating hash values instead of hashing transaction
    /// parts every time. Currently, all the hashes are recalculated from the scratch.
    pub fn construct<AcctId: Encode>(
        sighash: SigHash,
        tx: &Transaction<AcctId>,
        spending: &[TransactionOutput<AcctId>],
        index: u64,
        codesep_idx: u32,
    ) -> Self {
        let idx = index as usize;
        assert!(spending.len() == tx.inputs.len());
        assert!(idx < tx.inputs.len());

        Self {
            // Commit to the sighash mode
            sighash,

            // Inputs have three fields: outpoint, lock and witness. Witness is not committed to,
            // outpoints are included and locks are commited to by including the output being spent
            // into the message. The lock field is always fully determined by the output it spends.
            inputs: match sighash.input_mode() {
                InputMode::CommitWhoPays => TransactionInputSigMsg::CommitWhoPays {
                    outpoints: BlakeTwo256::hash_of(
                        &tx.inputs.iter().map(|i| &i.outpoint).collect::<Vec<&H256>>(),
                    ),
                    spending: BlakeTwo256::hash_of(&spending),
                    index,
                },
                InputMode::AnyoneCanPay => TransactionInputSigMsg::AnyoneCanPay {
                    outpoint: tx.inputs[idx].outpoint,
                    spending: BlakeTwo256::hash_of(&spending[idx]),
                },
            },

            // Outputs are comitted to as a whole, not individual fields.
            outputs: match sighash.output_mode() {
                OutputMode::All => TransactionOutputSigMsg::All {
                    outputs: BlakeTwo256::hash_of(&tx.outputs),
                },
                OutputMode::None => TransactionOutputSigMsg::None,
                OutputMode::Single => TransactionOutputSigMsg::Single {
                    output: tx.outputs.get(idx).map_or(H256::zero(), BlakeTwo256::hash_of),
                },
            },

            // Code separator position
            codesep_idx,
        }
    }
}

/// Signature schemes. Identified by the public key type.
pub trait Scheme: Sized {
    /// Signature type corresponding to the pubkey type for this scheme.
    type Signature: Decode;

    /// Verify signature against raw data.
    fn verify_raw(&self, sig: &Self::Signature, msg: &[u8]) -> bool;

    /// Parse signature & sighash and bundle it with a pubkey.
    fn parse_sig(self, sig: &[u8]) -> Option<SignatureDataFor<Self>> {
        let mut input = sig;
        let signature = Decode::decode(&mut input).ok()?;
        let sighash = match input {
            &[x] => SigHash::from_u8(x)?,
            &[] => SigHash::default(),
            _ => return None,
        };
        Some(SignatureDataFor {
            pubkey: self,
            signature,
            sighash,
        })
    }
}

// Schnorr signature scheme.
impl Scheme for sr25519::Public {
    type Signature = sr25519::Signature;

    fn verify_raw(&self, sig: &Self::Signature, msg: &[u8]) -> bool {
        crypto::sr25519_verify(sig, msg, self)
    }
}

/// A public key. An enum to accommodate for multiple signature schemes.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Encode, Decode, Debug, VariantCount)]
pub enum Public {
    /// Schnorr public key
    Schnorr(sr25519::Public),
}

impl Public {
    /// Parse a public key from given key byte array.
    pub fn parse(pk: &[u8]) -> ParseResult<Self> {
        let key_type = match pk.get(0) {
            Some(&kt) => kt,
            None => return ParseResult::Err,
        };
        if (key_type as usize) < Self::VARIANT_COUNT {
            Self::decode_all(&mut &pk).ok().into()
        } else {
            ParseResult::Reserved
        }
    }

    /// Parse signature data according to the current pubkey type.
    pub fn parse_sig(self, sig: &[u8]) -> Option<SignatureData> {
        match self {
            Public::Schnorr(pk) => pk.parse_sig(sig).map(SignatureData::Schnorr),
        }
    }
}

impl From<sr25519::Public> for Public {
    fn from(pk: sr25519::Public) -> Self {
        Self::Schnorr(pk)
    }
}

/// A signature together with its usage information for particular signature scheme.
pub struct SignatureDataFor<P: Scheme> {
    pubkey: P,
    signature: P::Signature,
    sighash: SigHash,
}

impl<P: Scheme> SignatureDataFor<P> {
    pub fn verify<T: Encode>(&self, msg: &T) -> bool {
        self.pubkey.verify_raw(&self.signature, &msg.encode())
    }
}

/// Signature data for multiple possible key types
pub enum SignatureData {
    Schnorr(SignatureDataFor<sr25519::Public>),
}

impl SignatureData {
    /// Verify signature against a message.
    pub fn verify<T: Encode>(&self, msg: &T) -> bool {
        match self {
            SignatureData::Schnorr(sd) => sd.verify(msg),
        }
    }

    /// Get sighash
    pub fn sighash(&self) -> SigHash {
        match self {
            SignatureData::Schnorr(s) => s.sighash,
        }
    }
}
