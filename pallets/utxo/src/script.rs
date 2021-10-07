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

use codec::DecodeAll;
use codec::{Decode, Encode};
use core::convert::TryInto;
use frame_support::sp_io::crypto;
use sp_core::sr25519;
use sp_std::prelude::*;
use variant_count::VariantCount;

/// An unvalidated signature type
type RawSignatureType = u8;

/// A signature together with its usage information
struct Signature {
    /// The raw signature data.
    signature: Vec<u8>,
    /// Sighash specifies which parts of the transaction are included in the hash that is verified
    /// by the signature.
    /// TODO: Sighash is not implemented at the moment, `SIGHASH_ALL` mode is used for everything.
    sighash: (),
}

/// A public key. An enum to accommodate for multiple signature schemes.
#[repr(u8)]
#[derive(Eq, PartialEq, Clone, Copy, Encode, Decode, Debug, VariantCount)]
enum Public {
    /// Schnorr public key
    Schnorr(sr25519::Public),
}

/// Mintlayer script context.
struct MLContext<'a> {
    tx_data: &'a [u8],
}

impl<'a> MLContext<'a> {
    fn new(tx_data: &'a [u8]) -> Self {
        MLContext { tx_data }
    }
}

impl chainscript::Context for MLContext<'_> {
    /// Maximum number of bytes pushable to the stack
    ///
    /// We do not limit the size of data pushed into the stack in Mintlayer since programmable pool
    /// binaries may be fairly large. Maximum size is still subject to `MAX_SCRIPT_SIZE`.
    const MAX_SCRIPT_ELEMENT_SIZE: usize = usize::MAX;

    /// Maximum number of public keys per multisig
    const MAX_PUBKEYS_PER_MULTISIG: usize = 20;

    /// Maximum script length in bytes
    ///
    /// Set it to 100kB for now to allow for mid-size smart contracts to be included in the script.
    const MAX_SCRIPT_SIZE: usize = 100 * 1024;

    /// Either parsed signature + metadata or unrecognized signature type.
    type Signature = Signature;

    /// Either a parsed public key or unrecognized pubkey type.
    type Public = Result<Public, RawSignatureType>;

    /// Extract a signature and sighash.
    fn parse_signature(&self, sig: &[u8]) -> Option<Self::Signature> {
        let (&_sighash_byte, sig) = sig.split_last()?;
        Some(Signature {
            signature: sig.to_vec(),
            sighash: (),
        })
    }

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
    fn parse_pubkey(&self, pk: &[u8]) -> Option<Self::Public> {
        let &key_type = pk.get(0)?;
        if (key_type as usize) < Public::VARIANT_COUNT {
            Public::decode_all(&mut &pk[..]).map(Ok).ok()
        } else {
            Some(Err(key_type))
        }
    }

    /// Verify signature.
    fn verify_signature(
        &self,
        sig: &Self::Signature,
        pk: &Self::Public,
        _subscript: &[u8],
    ) -> bool {
        let Signature {
            signature: sig,
            sighash: _sighash,
        } = sig;
        match pk {
            Ok(Public::Schnorr(pk)) => (&sig[..]).try_into().map_or(false, |sig| {
                crypto::sr25519_verify(&sr25519::Signature::from_raw(sig), self.tx_data, pk)
            }),
            // Unrecognized signature type => accept the signature.
            Err(_pk) => true,
        }
    }
}

/// Verify mintlayer script.
pub fn verify(tx_data: &[u8], witness: Vec<u8>, lock: Vec<u8>) -> chainscript::Result<()> {
    let ctx = MLContext::new(tx_data);
    chainscript::verify_witness_lock(&ctx, &witness.into(), &lock.into())
}

#[cfg(test)]
mod test {
    use super::*;
    use chainscript::Context;

    #[test]
    fn test_parse_pubkey() {
        let tx_data = [];
        let ctx = MLContext::new(&tx_data);
        let key = sr25519::Public::from_raw([42u8; 32]);
        let mut keydata = vec![0u8];
        keydata.extend(key.0.iter());
        assert_eq!(ctx.parse_pubkey(&keydata), Some(Ok(Public::Schnorr(key))));
        assert_eq!(ctx.parse_pubkey(&[42u8]), Some(Err(42u8)));
        assert_eq!(ctx.parse_pubkey(&[0u8, 1u8]), None);
    }
}
