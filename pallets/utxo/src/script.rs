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
use codec::Encode;
use sp_std::prelude::*;

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
mod test {
    use super::*;
    use chainscript::Context;
    use sp_core::sr25519;

    #[test]
    fn test_parse_pubkey() {
        let tx = Transaction::<u64> {
            inputs: vec![],
            outputs: vec![],
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
}
