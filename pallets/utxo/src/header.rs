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
// Author(s): C. Yap, Anton Sinitsyn

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::sp_std::convert::TryFrom;

use codec::{Decode, Encode};

pub type TXOutputHeader = u128;
pub type TokenID = u64;

// Check one bit in a number
#[inline(always)]
fn check_bit(number: u128, pos: u32) -> bool {
    (number & (1u128.overflowing_shl(pos).0)) != 0
}

#[inline(always)]
fn set_bit(number: u128, pos: u32) -> u128 {
    number | (1u128.overflowing_shl(pos).0)
}

// Copy number to bits field
fn fit_in_bits(number: u128, pos: u32, length: u32) -> u128 {
    let mut result = 0u128;
    for i in pos..pos + length {
        if check_bit(number, i) {
            result = set_bit(result, i - pos);
        }
    }
    result
}

fn move_bits(from: u128, f_offset: u32, f_length: u32, to_offset: u32) -> u128 {
    let mut result = 0u128;
    for i in f_offset..f_offset + f_length {
        if check_bit(from, i) {
            result = set_bit(result, i - f_offset + to_offset);
        }
    }
    result
}

#[derive(Debug)]
struct BitsField {
    length: u32,
    offset: u32,
    pub data: u128,
}

// Size of bit fields, total 72 bits
const SIGNATURE_METHOD_SIZE: u32 = 3;
const TOKEN_ID_SIZE: u32 = 64;
const VERSION_SIZE: u32 = 5;

#[derive(Debug)]
pub struct OutputHeader {
    sign_method: BitsField,
    token_id: BitsField,
    version: BitsField,
    reserve: BitsField,
}

impl OutputHeader {
    pub fn new(header: u128) -> OutputHeader {
        let mut offset = 0;

        // Signature method
        let sign_method = BitsField {
            length: SIGNATURE_METHOD_SIZE,
            offset,
            data: fit_in_bits(header, offset, SIGNATURE_METHOD_SIZE),
        };
        offset += SIGNATURE_METHOD_SIZE;

        // Token ID
        let token_id = BitsField {
            length: TOKEN_ID_SIZE,
            offset,
            data: fit_in_bits(header, offset, TOKEN_ID_SIZE),
        };
        offset += TOKEN_ID_SIZE;

        // Version number
        let version = BitsField {
            length: VERSION_SIZE,
            offset,
            data: fit_in_bits(header, offset, VERSION_SIZE),
        };
        offset += VERSION_SIZE;

        // You can add another field here. Just do not forget to add offset
        OutputHeader {
            sign_method,
            token_id,
            version,
            reserve: BitsField {
                length: u128::BITS - offset,
                offset,
                data: fit_in_bits(header, offset, u128::BITS - offset),
            },
        }
    }

    pub fn as_u128(&self) -> u128 {
        // Easy one because these bits have a concrete place
        let mut result = 0u128;
        let mut offset = 0;
        result += move_bits(self.sign_method.data, 0, SIGNATURE_METHOD_SIZE, offset);
        offset += SIGNATURE_METHOD_SIZE;
        result += move_bits(self.token_id.data, 0, TOKEN_ID_SIZE, offset);
        offset += TOKEN_ID_SIZE;
        result += move_bits(self.version.data, 0, VERSION_SIZE, offset);

        result
    }

    pub fn sign_method(&self) -> Option<SignatureMethod> {
        TryFrom::try_from(self.sign_method.data).ok()
    }

    pub fn set_sign_method(&mut self, sign_method: SignatureMethod) {
        self.sign_method.data = sign_method as u128;
    }

    pub fn token_id(&self) -> TokenID {
        self.token_id.data as u64
    }

    pub fn set_token_id(&mut self, token_id: TokenID) {
        self.token_id.data = token_id as u128;
    }

    pub fn token_type(&self) -> Option<TokenType> {
        TryFrom::try_from(self.token_id.data).ok()
    }

    pub fn set_token_type(&mut self, token_id: TokenType) {
        self.token_id.data = token_id as u128;
    }

    pub fn version(&self) -> u128 {
        self.version.data
    }

    pub fn set_version(&mut self, version: u64) {
        self.version.data = version as u128;
    }

    pub fn validate(&self) -> bool {
        self.token_type().is_some() & self.sign_method().is_some()
    }
}

pub trait OutputHeaderHelper {
    fn as_tx_output_header(&self) -> OutputHeader;
}

impl OutputHeaderHelper for TXOutputHeader {
    fn as_tx_output_header(&self) -> OutputHeader {
        OutputHeader::new(*self)
    }
}

// https://stackoverflow.com/posts/57578431/revisions from Shepmaster
// whenever a new type/variant is supported, we don't have to code a lot of 'matches' boilerplate.
macro_rules! u128_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl TryFrom<u128> for $name {
            type Error = &'static str;

            fn try_from(v: u128) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u128 => Ok($name::$vname),)*
                    _ => {
                        Err(stringify!(unsupported $name))
                    },
                }
            }
        }
    }
}

u128_to_enum! {
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    #[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
    pub enum SignatureMethod {
        BLS = 0,
        Schnorr = 1,
        ZkSnark = 2,
    }
}

u128_to_enum! {
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub enum TokenType {
    MLT = 0,
    BTC = 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate() {
        // improper sig meth
        assert_eq!(OutputHeader::new(0b11111_011u128).validate(), false);
        // improper token type
        assert_eq!(OutputHeader::new(0b11000_000u128).validate(), false);

        // Proper header
        assert!(OutputHeader::new(
            0b10_0000000000000000000000000000000000000000000000000000000000000000_010u128
        )
        .validate());
        assert!(OutputHeader::new(
            0b01_0000000000000000000000000000000000000000000000000000000000000001_000u128
        )
        .validate());
        assert!(OutputHeader::new(0u128).validate());
    }

    #[test]
    fn signatures() {
        let x = 0b11011_000u128; // last 3 bits are 000, so signature should be 0 or BLS.
        let header = OutputHeader::new(x);
        assert!(header.sign_method().is_some());
        assert_eq!(header.sign_method().unwrap(), SignatureMethod::BLS);

        let x = 0b0000100_001; // last 3 bits are 001, so signature should be Schnorr
        assert_eq!(
            OutputHeader::new(x).sign_method().unwrap(),
            SignatureMethod::Schnorr
        );

        let x = 0b111110_010; // last 3 bits are 010, so signature should be ZkSnark
        assert_eq!(
            OutputHeader::new(x).sign_method().unwrap(),
            SignatureMethod::ZkSnark
        );

        let x = 0b10_111; // last 3 bits is are, and it's not yet supported.
        assert_eq!(OutputHeader::new(x).sign_method(), None);

        let mut header = OutputHeader::new(185u128); // last 3 bits are 001. Convert to 000 for BLS.
        header.set_sign_method(SignatureMethod::BLS);
        assert_eq!(header.as_u128(), 184);

        // last 3 bits of header are 000. Convert to 010 for ZkSnark.
        header.set_sign_method(SignatureMethod::ZkSnark);
        assert_eq!(header.as_u128(), 186);
    }

    #[allow(dead_code)]
    fn print_bits(number: u128) {
        let mut space = 0;
        for i in 0..128 {
            if check_bit(number, 127 - i) {
                print!("1");
            } else {
                print!("0");
            }
            space += 1;
            if space == 4 {
                space = 0;
                print!("_");
            }
        }
        println!("");
    }

    #[test]
    fn token_types() {
        // the middle 64 bits are 000000, so type is MLT.
        let header = OutputHeader::new(
            0b1010_0000000000000000000000000000000000000000000000000000000000000000_110,
        );
        assert!(header.token_type().is_some());
        assert_eq!(header.token_type().unwrap(), TokenType::MLT);

        // the middle 64 bits are 000001, so type is BTC.
        let header = OutputHeader::new(
            0b1010_0000000000000000000000000000000000000000000000000000000000000001_110,
        );
        assert!(header.token_type().is_some());
        assert_eq!(header.token_type().unwrap(), TokenType::BTC);

        // the first 64 bits are 000010, so type is BTC.
        assert_eq!(
            OutputHeader::new(0b000001_101).token_type().unwrap(),
            TokenType::BTC
        );
        assert_eq!(
            OutputHeader::new(3u128).token_type().unwrap(),
            TokenType::MLT
        );
        assert_eq!(OutputHeader::new(0b110001_000).token_type(), None);

        let mut improper_header = OutputHeader::new(321u128); // 101000_001, and must be converted to 10_001.
        improper_header.set_token_type(TokenType::BTC);
        assert_eq!(improper_header.as_u128(), 0b000000001_001);

        improper_header = OutputHeader::new(178u128); // 10110_010, and must be converted to 000000_010 or 2.
        improper_header.set_token_type(TokenType::MLT);
        assert_eq!(improper_header.as_u128(), 2);
    }
}
