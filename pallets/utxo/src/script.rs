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

use crate::{sign, Transaction, TransactionOutput};
use chainscript::context::ParseResult;
use codec::{Decode, Encode};
use core::time::Duration;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::prelude::*;

/// Blockchain time as encoded on chain.
#[derive(Eq, PartialEq, Clone, Copy, Encode, Decode, Debug, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct RawBlockTime(#[codec(compact)] u64);

impl RawBlockTime {
    // Values less than this constant are considered to be a block number, other values are
    // interpreted as UNIX time stamp. This constant is identical to what is used in Bitcoin.
    // At 1min/block, the block numbers run out in about 900 years after the genesis block.
    const THRESHOLD: u64 = 500_000_000u64;

    /// Create a new raw time lock
    pub fn new(tl: u64) -> Self {
        Self(tl)
    }

    /// Get the lock time as a u64
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Does this time lock restrict blocks?
    pub fn is_blocks(&self) -> bool {
        self.0 < Self::THRESHOLD
    }

    /// Get block time
    pub fn time(&self) -> BlockTime {
        if self.is_blocks() {
            BlockTime::Blocks(self.0 as u32)
        } else {
            // Seconds need to be converted to milliseconds
            BlockTime::Timestamp(Duration::from_secs(self.0))
        }
    }
}

/// Represents a point in blockchain time, either in number of blocks or in real world time.
#[derive(Eq, PartialEq, Clone, Copy)]
pub enum BlockTime {
    /// Number of blocks since genesis
    Blocks(u32),
    /// Real world time
    Timestamp(Duration),
}

impl BlockTime {
    #[cfg(test)]
    pub fn as_raw(&self) -> Option<RawBlockTime> {
        match *self {
            Self::Blocks(b) => Some(RawBlockTime(b as u64)).filter(|r| r.is_blocks()),
            Self::Timestamp(t) => Some(RawBlockTime(t.as_secs())).filter(|r| !r.is_blocks()),
        }
    }
}

impl PartialOrd for BlockTime {
    fn partial_cmp(&self, rhs: &Self) -> Option<core::cmp::Ordering> {
        match (self, rhs) {
            (Self::Blocks(a), Self::Blocks(b)) => a.partial_cmp(b),
            (Self::Timestamp(a), Self::Timestamp(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

/// Mintlayer script context.
struct MLContext<'a, AccountId> {
    tx: &'a Transaction<AccountId>,
    utxos: &'a [TransactionOutput<AccountId>],
    index: u64,
}

impl<'a, AccountId: 'a + Encode> chainscript::Context for MLContext<'a, AccountId> {
    /// Maximum script length in bytes
    ///
    /// Set it to 100kB for now to allow for mid-size smart contracts to be included in the script.
    const MAX_SCRIPT_SIZE: usize = 100 * 1024;

    /// Maximum number of bytes pushable to the stack
    ///
    /// We do not limit the size of data pushed into the stack in Mintlayer since programmable pool
    /// binaries may be fairly large.
    const MAX_SCRIPT_ELEMENT_SIZE: usize = Self::MAX_SCRIPT_SIZE;

    /// Maximum number of public keys per multisig
    const MAX_PUBKEYS_PER_MULTISIG: usize = 20;

    /// Either a parsed public key or unrecognized pubkey type.
    type Public = sign::Public;

    /// Either parsed signature + metadata or unrecognized signature type.
    type SignatureData = sign::SignatureData;

    /// Extract a pubkey and check it is in the correct format.
    ///
    /// Explanation of return values is analogous to `parse_signature`.
    ///
    /// The function returns a Rsult wrapped in an Option type.
    /// * `None` represents a parsing failure, the transaction is rejected.
    /// * `Some(Err(x))` represents an unrecognized pubkey type with type ID `x`.
    ///   The pubkey with unknown type ID always succeeds validation. This allows the type to be
    ///   allocated later for a new signature scheme without introducing a hard fork.
    /// * `Some(Ok(pk))` is a successfully processed key.
    fn parse_pubkey(&self, pk: &[u8]) -> ParseResult<Self::Public> {
        Self::Public::parse(pk)
    }

    /// Extract a signature and sighash.
    fn parse_signature(&self, pk: Self::Public, sig: &[u8]) -> Option<Self::SignatureData> {
        pk.parse_sig(sig)
    }

    /// Verify signature.
    fn verify_signature(&self, sd: &Self::SignatureData, _: &[u8], sep_idx: u32) -> bool {
        use sign::TransactionSigMsg as Msg;
        let msg = Msg::construct(sd.sighash(), &self.tx, self.utxos, self.index, sep_idx);
        sd.verify(&msg)
    }

    /// Verify absolute time lock.
    fn check_lock_time(&self, time: i64) -> bool {
        time >= 0 && self.tx.time_lock.time() >= RawBlockTime::new(time as u64).time()
    }
}

/// Verify mintlayer script.
pub fn verify<AccountId: Encode>(
    tx: &Transaction<AccountId>,
    utxos: &[TransactionOutput<AccountId>],
    index: u64,
    witness: Vec<u8>,
    lock: Vec<u8>,
) -> chainscript::Result<()> {
    let ctx = MLContext { tx, utxos, index };
    chainscript::verify_witness_lock(&ctx, &witness.into(), &lock.into())
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use chainscript::Context;
    use core::time::Duration;
    use proptest::prelude::*;
    use sp_core::sr25519;

    // Generate block time in seconds
    pub fn gen_block_time_real() -> impl Strategy<Value = RawBlockTime> {
        (RawBlockTime::THRESHOLD..3 * RawBlockTime::THRESHOLD).prop_map(RawBlockTime::new)
    }

    #[test]
    fn test_parse_pubkey() {
        let tx = Transaction::<u64> {
            inputs: vec![],
            outputs: vec![],
            time_lock: Default::default(),
        };
        let ctx = MLContext {
            tx: &tx,
            utxos: &[],
            index: 0,
        };
        let key = sr25519::Public::from_raw([42u8; 32]);
        let mut keydata = vec![0u8];
        keydata.extend(key.0.iter());
        assert_eq!(ctx.parse_pubkey(&keydata), ParseResult::Ok(key.into()));
        assert_eq!(ctx.parse_pubkey(&[42u8]), ParseResult::Reserved);
        assert_eq!(ctx.parse_pubkey(&[0u8, 1u8]), ParseResult::Err);
    }

    #[test]
    fn test_time_lock_type_mismatch() {
        let tx = Transaction::<u64> {
            inputs: Vec::new(),
            outputs: Vec::new(),
            time_lock: BlockTime::Timestamp(Duration::from_secs(1_000_000_000)).as_raw().unwrap(),
        };
        let ctx = MLContext {
            tx: &tx,
            utxos: &[],
            index: 0,
        };
        let script = chainscript::Builder::new()
            .push_int(BlockTime::Blocks(5).as_raw().unwrap().as_u64() as i64)
            .push_opcode(chainscript::opcodes::all::OP_CLTV)
            .into_script();
        let result = chainscript::run_script(&ctx, &script, Vec::new().into());
        assert_eq!(result, Err(chainscript::Error::TimeLock));
    }
}
