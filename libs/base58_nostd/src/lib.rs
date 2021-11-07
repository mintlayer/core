// Copyright (c) 2021 RBB S.r.l
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
// Based on https://github.com/trezor/trezor-crypto/blob/master/base58.c
// commit hash: c6e7d37
// license: MIT
// works only up to 128 bytes

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::sp_io::hashing::sha2_256;
use sp_std::vec;
use sp_std::vec::Vec;

pub const TOKEN_ID_PREFIX: u8 = b"M"[0];

const BASE58_ALPHABET: &'static [u8] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

const B58_BITCOIN_DIGITS_MAP: &'static [i8] = &[
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, -1, -1, -1, -1, -1, -1, -1, 9, 10, 11, 12, 13, 14, 15, 16, -1,
    17, 18, 19, 20, 21, -1, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, -1, -1, -1, -1, -1, -1, 33,
    34, 35, 36, 37, 38, 39, 40, 41, 42, 43, -1, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56,
    57, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

/// Errors that can occur when decoding base58 encoded string.
#[derive(Debug, PartialEq)]
pub enum FromBase58Error {
    /// The input contained a character which is not a part of the base58 format.
    InvalidBase58Character(char, usize),
    /// The input had invalid length.
    InvalidBase58Length,
    /// Base58 string contains invalid checksum
    InvalidChecksum,
    /// The input has invalid prefix.
    InvalidPrefix,
}

/// A trait for converting a value to base58 encoded string.
pub trait ToBase58 {
    /// Converts a value of `self` to a base58 value, returning the owned string.
    fn to_base58(&self) -> Vec<u8>;
    /// Converts a value of `self` to a base58 value with checksum applied, returning the owned string.
    fn to_mls_b58check(&self, prefix: Option<Vec<u8>>) -> Vec<u8>;
}

/// A trait for converting base58 encoded values.
pub trait FromBase58 {
    /// Convert a value of `self`, interpreted as base58 encoded data, into an owned vector of bytes, returning a vector.
    fn from_base58(&self) -> Result<Vec<u8>, FromBase58Error>;
    /// Converts a value of `self`, interpreted as base58 encoded data with checksum applied, into an owned vector of bytes,
    /// returning a vector.
    fn from_mls_b58check(&self, prefix: Option<Vec<u8>>) -> Result<Vec<u8>, FromBase58Error>;
}

fn checksum(payload: &[u8]) -> Vec<u8> {
    let sha256 = sha2_256(payload);
    let doubled_sha256 = sha2_256(&sha256);
    // Return the first 4 bytes of sha256(sha256(payload))
    Vec::from(&doubled_sha256[..4])
}

fn encode_to_base58(payload: &[u8]) -> Vec<u8> {
    let zcount = payload.iter().take_while(|x| **x == 0).count();
    let size = (payload.len() - zcount) * 138 / 100 + 1;
    let mut buffer = vec![0u8; size];
    let mut i = zcount;
    let mut high = size - 1;
    while i < payload.len() {
        let mut carry = payload[i] as u32;
        let mut j = size - 1;

        while j > high || carry != 0 {
            carry += 256 * buffer[j] as u32;
            buffer[j] = (carry % 58) as u8;
            carry /= 58;
            if j > 0 {
                j -= 1;
            }
        }
        i += 1;
        high = j;
    }
    let mut j = buffer.iter().take_while(|x| **x == 0).count();
    let mut result = Vec::new();
    for _ in 0..zcount {
        result.push(b'1');
    }
    while j < size {
        result.push(BASE58_ALPHABET[buffer[j] as usize]);
        j += 1;
    }
    result
}

fn decode_from_base58(payload: &str) -> Result<Vec<u8>, FromBase58Error> {
    let mut bin = [0u8; 132];
    let mut out = [0u32; (132 + 3) / 4];
    let bytesleft = (bin.len() % 4) as u8;
    let zeromask = match bytesleft {
        0 => 0u32,
        _ => 0xffffffff << (bytesleft * 8),
    };

    let zcount = payload.chars().take_while(|x| *x == '1').count();
    let mut i = zcount;
    let b58: Vec<u8> = payload.bytes().collect();

    while i < payload.len() {
        if (b58[i] & 0x80) != 0 {
            // High-bit set on invalid digit
            return Err(FromBase58Error::InvalidBase58Character(b58[i] as char, i));
        }

        if B58_BITCOIN_DIGITS_MAP[b58[i] as usize] == -1 {
            // // Invalid base58 digit
            return Err(FromBase58Error::InvalidBase58Character(b58[i] as char, i));
        }

        let mut c = B58_BITCOIN_DIGITS_MAP[b58[i] as usize] as u64;
        let mut j = out.len();
        while j != 0 {
            j -= 1;
            let t = out[j] as u64 * 58 + c;
            c = (t & 0x3f00000000) >> 32;
            out[j] = (t & 0xffffffff) as u32;
        }

        if c != 0 {
            // Output number too big (carry to the next int32)
            return Err(FromBase58Error::InvalidBase58Length);
        }

        if (out[0] & zeromask) != 0 {
            // Output number too big (last int32 filled too far)
            return Err(FromBase58Error::InvalidBase58Length);
        }

        i += 1;
    }

    let mut i = 1;
    let mut j = 0;

    bin[0] = match bytesleft {
        3 => ((out[0] & 0xff0000) >> 16) as u8,
        2 => ((out[0] & 0xff00) >> 8) as u8,
        1 => {
            j = 1;
            (out[0] & 0xff) as u8
        }
        _ => {
            i = 0;
            bin[0]
        }
    };

    while j < out.len() {
        bin[i] = ((out[j] >> 0x18) & 0xff) as u8;
        bin[i + 1] = ((out[j] >> 0x10) & 0xff) as u8;
        bin[i + 2] = ((out[j] >> 8) & 0xff) as u8;
        bin[i + 3] = ((out[j] >> 0) & 0xff) as u8;
        i += 4;
        j += 1;
    }

    let leading_zeros = bin.iter().take_while(|x| **x == 0).count();
    Ok(bin[leading_zeros - zcount..].to_vec())
}

impl FromBase58 for str {
    fn from_base58(&self) -> Result<Vec<u8>, FromBase58Error> {
        decode_from_base58(self)
    }

    fn from_mls_b58check(&self, prefix: Option<Vec<u8>>) -> Result<Vec<u8>, FromBase58Error> {
        let mut payload: Vec<u8> = self.from_base58()?;
        if payload.len() < 5 {
            return Err(FromBase58Error::InvalidChecksum);
        }
        let checksum_index = payload.len() - 4;
        let provided_checksum = payload.split_off(checksum_index);
        let checksum = checksum(&payload).to_vec();
        if checksum != provided_checksum {
            return Err(FromBase58Error::InvalidChecksum);
        }
        if let Some(ref prefix) = prefix {
            let payload_prefix = payload[..prefix.len()].to_vec();
            // Let's check is it right prefix or not
            if &payload_prefix != prefix {
                Err(FromBase58Error::InvalidPrefix)?;
            }
        }
        match prefix {
            Some(prefix) => Ok(payload[prefix.len()..].to_vec()),
            None => Ok(payload),
        }
    }
}

impl ToBase58 for [u8] {
    fn to_base58(&self) -> Vec<u8> {
        encode_to_base58(self)
    }

    fn to_mls_b58check(&self, prefix: Option<Vec<u8>>) -> Vec<u8> {
        let mut payload = match prefix {
            Some(prefix) => prefix.clone(),
            None => vec![],
        };
        // let mut payload = vec![prefix];
        payload.extend(self);
        payload.extend(checksum(payload.as_slice()));
        encode_to_base58(payload.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::{FromBase58, FromBase58Error, ToBase58, TOKEN_ID_PREFIX};

    #[test]
    fn test_from_base58_basic() {
        assert_eq!("".from_base58().unwrap(), b"");
        assert_eq!("Z".from_base58().unwrap(), &[32]);
        assert_eq!("n".from_base58().unwrap(), &[45]);
        assert_eq!("q".from_base58().unwrap(), &[48]);
        assert_eq!("r".from_base58().unwrap(), &[49]);
        assert_eq!("z".from_base58().unwrap(), &[57]);
        assert_eq!("4SU".from_base58().unwrap(), &[45, 49]);
        assert_eq!("4k8".from_base58().unwrap(), &[49, 49]);
        assert_eq!("ZiCa".from_base58().unwrap(), &[97, 98, 99]);
        assert_eq!("3mJr7AoUXx2Wqd".from_base58().unwrap(), b"1234598760");
        assert_eq!(
            "3yxU3u1igY8WkgtjK92fbJQCd4BZiiT1v25f".from_base58().unwrap(),
            b"abcdefghijklmnopqrstuvwxyz"
        );
    }

    #[test]
    fn test_from_base58_invalid_char() {
        assert!("0".from_base58().is_err());
        assert!("O".from_base58().is_err());
        assert!("I".from_base58().is_err());
        assert!("l".from_base58().is_err());
        assert!("3mJr0".from_base58().is_err());
        assert!("O3yxU".from_base58().is_err());
        assert!("3sNI".from_base58().is_err());
        assert!("4kl8".from_base58().is_err());
        assert!("s!5<".from_base58().is_err());
        assert!("t$@mX<*".from_base58().is_err());
    }

    #[test]
    fn test_from_base58_initial_zeros() {
        assert_eq!("1ZiCa".from_base58().unwrap(), b"\0abc");
        assert_eq!("11ZiCa".from_base58().unwrap(), b"\0\0abc");
        assert_eq!("111ZiCa".from_base58().unwrap(), b"\0\0\0abc");
        assert_eq!("1111ZiCa".from_base58().unwrap(), b"\0\0\0\0abc");
    }

    #[test]
    fn test_to_base58_basic() {
        assert_eq!(b"".to_base58(), "".as_bytes());
        assert_eq!(&[32].to_base58(), "Z".as_bytes());
        assert_eq!(&[45].to_base58(), "n".as_bytes());
        assert_eq!(&[48].to_base58(), "q".as_bytes());
        assert_eq!(&[49].to_base58(), "r".as_bytes());
        assert_eq!(&[57].to_base58(), "z".as_bytes());
        assert_eq!(&[45, 49].to_base58(), "4SU".as_bytes());
        assert_eq!(&[49, 49].to_base58(), "4k8".as_bytes());
        assert_eq!(b"abc".to_base58(), "ZiCa".as_bytes());
        assert_eq!(b"1234598760".to_base58(), "3mJr7AoUXx2Wqd".as_bytes());
        assert_eq!(
            b"abcdefghijklmnopqrstuvwxyz".to_base58(),
            "3yxU3u1igY8WkgtjK92fbJQCd4BZiiT1v25f".as_bytes()
        );
    }

    #[test]
    fn test_to_base58_initial_zeros() {
        assert_eq!(b"\0abc".to_base58(), "1ZiCa".as_bytes());
        assert_eq!(b"\0\0abc".to_base58(), "11ZiCa".as_bytes());
        assert_eq!(b"\0\0\0abc".to_base58(), "111ZiCa".as_bytes());
        assert_eq!(b"\0\0\0\0abc".to_base58(), "1111ZiCa".as_bytes());
    }

    #[test]
    fn test_from_base58_compatible_functional_tests() {
        // The data was being prepared in python script

        assert_eq!(
            "2QjRKB7mHaXRjhUmgcQGAbDHPre2Uvq9ev4YiiFgLoUPrQdB52MuHoRwmB"
                .from_base58()
                .unwrap(),
            b"To be, or not to be, that is the question:"
        );

        assert_eq!(
            "LApxNT84PpjfwjYZyDdhQTNAuEp28SssymbKcj68fEc7wLh2qpkpXAuf"
                .from_base58()
                .unwrap(),
            b"Whether 'tis nobler in the mind to suffer"
        );

        assert_eq!(
            "USm3fpdSjgtutT9UNHZgsaR4UBcHmgYfxcaVubFjhj9Tio5Nfq9XNV5puD7H"
                .from_base58()
                .unwrap(),
            b"The slings and arrows of outrageous fortune,"
        );

        assert_eq!(
            "JRYvHV9zVEFpwLXQLjTs8VhnP1nPiBZUFdHA5into6ntyEPsLwpnR8Vp"
                .from_base58()
                .unwrap(),
            b"Or to take arms against a sea of troubles"
        );

        assert_eq!(
            "26LXuFRSRgp2fUf8QhNjeEHjniK599smzB7pJsqf1XpLS9bkgd4d7gM9UX"
                .from_base58()
                .unwrap(),
            b"And by opposing end them. To die-to sleep,"
        );

        assert_eq!(
            "3dU1LpdBTnUsha3T3cGiEUZPTtzRfLhCA83k22CMvbzKV9oMb87".from_base58().unwrap(),
            b"No more; and by a sleep to say we end"
        );

        assert_eq!(
            "ADeyMxyacx916HoiijiCJRMqdjtWULxSE2eSz1t11rQbLSvVbhv6cCiwqKFAQav"
                .from_base58()
                .unwrap(),
            b"The heart-ache and the thousand natural shocks"
        );

        assert_eq!(
            "2QhwWNuP7oGHaHRjydcvqxLC31wKkZ12MWFBoXpe1wLJ15z6vSRuqUdNYd"
                .from_base58()
                .unwrap(),
            b"That flesh is heir to: 'tis a consummation"
        );

        assert_eq!(
            "4Q7Mny7G48TgtAU6u3eqhT7FDqALB7LZ466AThn4G9jv7BBhx9pXbJz".from_base58().unwrap(),
            b"Devoutly to be wish'd. To die, to sleep;"
        );

        assert_eq!(
            "Efu1HHgBffNXqXSgamBAvVNBN28JgEtp2QBqZsTRvbn44DQFEL2YfVYnFrPAdBcEz25"
                .from_base58()
                .unwrap(),
            b"To sleep, perchance to dream-ay, there's the rub:"
        );
    }

    #[test]
    fn test_to_base58_compatible_functional_tests() {
        // The data was being prepared in python script

        assert_eq!(
            b"To be, or not to be, that is the question:".to_base58(),
            "2QjRKB7mHaXRjhUmgcQGAbDHPre2Uvq9ev4YiiFgLoUPrQdB52MuHoRwmB".as_bytes()
        );

        assert_eq!(
            b"Whether 'tis nobler in the mind to suffer".to_base58(),
            "LApxNT84PpjfwjYZyDdhQTNAuEp28SssymbKcj68fEc7wLh2qpkpXAuf".as_bytes()
        );

        assert_eq!(
            b"The slings and arrows of outrageous fortune,".to_base58(),
            "USm3fpdSjgtutT9UNHZgsaR4UBcHmgYfxcaVubFjhj9Tio5Nfq9XNV5puD7H".as_bytes()
        );

        assert_eq!(
            b"Or to take arms against a sea of troubles".to_base58(),
            "JRYvHV9zVEFpwLXQLjTs8VhnP1nPiBZUFdHA5into6ntyEPsLwpnR8Vp".as_bytes()
        );

        assert_eq!(
            b"And by opposing end them. To die-to sleep,".to_base58(),
            "26LXuFRSRgp2fUf8QhNjeEHjniK599smzB7pJsqf1XpLS9bkgd4d7gM9UX".as_bytes()
        );

        assert_eq!(
            b"No more; and by a sleep to say we end".to_base58(),
            "3dU1LpdBTnUsha3T3cGiEUZPTtzRfLhCA83k22CMvbzKV9oMb87".as_bytes()
        );

        assert_eq!(
            b"The heart-ache and the thousand natural shocks".to_base58(),
            "ADeyMxyacx916HoiijiCJRMqdjtWULxSE2eSz1t11rQbLSvVbhv6cCiwqKFAQav".as_bytes()
        );

        assert_eq!(
            b"That flesh is heir to: 'tis a consummation".to_base58(),
            "2QhwWNuP7oGHaHRjydcvqxLC31wKkZ12MWFBoXpe1wLJ15z6vSRuqUdNYd".as_bytes()
        );

        assert_eq!(
            b"Devoutly to be wish'd. To die, to sleep;".to_base58(),
            "4Q7Mny7G48TgtAU6u3eqhT7FDqALB7LZ466AThn4G9jv7BBhx9pXbJz".as_bytes()
        );

        assert_eq!(
            b"To sleep, perchance to dream-ay, there's the rub:".to_base58(),
            "Efu1HHgBffNXqXSgamBAvVNBN28JgEtp2QBqZsTRvbn44DQFEL2YfVYnFrPAdBcEz25".as_bytes()
        );
    }

    #[test]
    fn to_base58check() {
        assert_eq!(
            b"SOME_TOKEN_ID".to_mls_b58check(Some(vec![TOKEN_ID_PREFIX])),
            "4D27mSFWbKGNea2eGBpjuCbEy".as_bytes()
        );

        // Took from js library:
        // https://github.com/wzbg/base58check/blob/master/test.js

        assert_eq!(
            [
                0xf5, 0xf2, 0xd6, 0x24, 0xcf, 0xb5, 0xc3, 0xf6, 0x6d, 0x06, 0x12, 0x3d, 0x08, 0x29,
                0xd1, 0xc9, 0xce, 0xbf, 0x77, 0x0e
            ]
            .to_mls_b58check(Some(vec![0])),
            "1PRTTaJesdNovgne6Ehcdu1fpEdX7913CK".as_bytes()
        );

        assert_eq!(
            [
                0x1E, 0x99, 0x42, 0x3A, 0x4E, 0xD2, 0x76, 0x08, 0xA1, 0x5A, 0x26, 0x16, 0xA2, 0xB0,
                0xE9, 0xE5, 0x2C, 0xED, 0x33, 0x0A, 0xC5, 0x30, 0xED, 0xCC, 0x32, 0xC8, 0xFF, 0xC6,
                0xA5, 0x26, 0xAE, 0xDD,
            ]
            .to_mls_b58check(Some(vec![0x80])),
            "5J3mBbAH58CpQ3Y5RNJpUKPE62SQ5tfcvU2JpbnkeyhfsYB1Jcn".as_bytes()
        );

        assert_eq!(
            [
                0x27, 0xb5, 0x89, 0x1b, 0x01, 0xda, 0x2d, 0xb7, 0x4c, 0xde, 0x16, 0x89, 0xa9, 0x7a,
                0x2a, 0xcb, 0xe2, 0x3d, 0x5f, 0xb1
            ]
            .to_mls_b58check(Some(vec![0])),
            "14cxpo3MBCYYWCgF74SWTdcmxipnGUsPw3".as_bytes()
        );

        assert_eq!(
            [
                0x3a, 0xba, 0x41, 0x62, 0xc7, 0x25, 0x1c, 0x89, 0x12, 0x07, 0xb7, 0x47, 0x84, 0x05,
                0x51, 0xa7, 0x19, 0x39, 0xb0, 0xde, 0x08, 0x1f, 0x85, 0xc4, 0xe4, 0x4c, 0xf7, 0xc1,
                0x3e, 0x41, 0xda, 0xa6
            ]
            .to_mls_b58check(Some(vec![0x80])),
            "5JG9hT3beGTJuUAmCQEmNaxAuMacCTfXuw1R3FCXig23RQHMr4K".as_bytes()
        );

        assert_eq!(
            [
                0x08, 0x6e, 0xaa, 0x67, 0x78, 0x95, 0xf9, 0x2d, 0x4a, 0x6c, 0x5e, 0xf7, 0x40, 0xc1,
                0x68, 0x93, 0x2b, 0x5e, 0x3f, 0x44
            ]
            .to_mls_b58check(Some(vec![0])),
            "1mayif3H2JDC62S4N3rLNtBNRAiUUP99k".as_bytes()
        );

        assert_eq!(
            [
                0xed, 0xdb, 0xdc, 0x11, 0x68, 0xf1, 0xda, 0xea, 0xdb, 0xd3, 0xe4, 0x4c, 0x1e, 0x3f,
                0x8f, 0x5a, 0x28, 0x4c, 0x20, 0x29, 0xf7, 0x8a, 0xd2, 0x6a, 0xf9, 0x85, 0x83, 0xa4,
                0x99, 0xde, 0x5b, 0x19
            ]
            .to_mls_b58check(Some(vec![0x80])),
            "5Kd3NBUAdUnhyzenEwVLy9pBKxSwXvE9FMPyR4UKZvpe6E3AgLr".as_bytes()
        );
    }

    #[test]
    fn from_base58check() {
        assert_eq!(
            "3vQB7B6MrGQZaxCuFg4oh".from_mls_b58check(None).unwrap(),
            b"hello world".to_vec()
        );

        // Took from js library:
        // https://github.com/wzbg/base58check/blob/master/test.js

        assert_eq!(
            "1PRTTaJesdNovgne6Ehcdu1fpEdX7913CK".from_mls_b58check(Some(vec![0])).unwrap(),
            vec![
                0xf5, 0xf2, 0xd6, 0x24, 0xcf, 0xb5, 0xc3, 0xf6, 0x6d, 0x06, 0x12, 0x3d, 0x08, 0x29,
                0xd1, 0xc9, 0xce, 0xbf, 0x77, 0x0e
            ]
        );

        assert_eq!(
            "5J3mBbAH58CpQ3Y5RNJpUKPE62SQ5tfcvU2JpbnkeyhfsYB1Jcn"
                .from_mls_b58check(Some(vec![0x80]))
                .unwrap(),
            vec![
                0x1E, 0x99, 0x42, 0x3A, 0x4E, 0xD2, 0x76, 0x08, 0xA1, 0x5A, 0x26, 0x16, 0xA2, 0xB0,
                0xE9, 0xE5, 0x2C, 0xED, 0x33, 0x0A, 0xC5, 0x30, 0xED, 0xCC, 0x32, 0xC8, 0xFF, 0xC6,
                0xA5, 0x26, 0xAE, 0xDD,
            ]
        );

        assert_eq!(
            "14cxpo3MBCYYWCgF74SWTdcmxipnGUsPw3".from_mls_b58check(Some(vec![0])).unwrap(),
            vec![
                0x27, 0xb5, 0x89, 0x1b, 0x01, 0xda, 0x2d, 0xb7, 0x4c, 0xde, 0x16, 0x89, 0xa9, 0x7a,
                0x2a, 0xcb, 0xe2, 0x3d, 0x5f, 0xb1
            ]
        );

        assert_eq!(
            "5JG9hT3beGTJuUAmCQEmNaxAuMacCTfXuw1R3FCXig23RQHMr4K"
                .from_mls_b58check(Some(vec![0x80]))
                .unwrap(),
            vec![
                0x3a, 0xba, 0x41, 0x62, 0xc7, 0x25, 0x1c, 0x89, 0x12, 0x07, 0xb7, 0x47, 0x84, 0x05,
                0x51, 0xa7, 0x19, 0x39, 0xb0, 0xde, 0x08, 0x1f, 0x85, 0xc4, 0xe4, 0x4c, 0xf7, 0xc1,
                0x3e, 0x41, 0xda, 0xa6
            ]
        );

        assert_eq!(
            "1mayif3H2JDC62S4N3rLNtBNRAiUUP99k".from_mls_b58check(Some(vec![0])).unwrap(),
            vec![
                0x08, 0x6e, 0xaa, 0x67, 0x78, 0x95, 0xf9, 0x2d, 0x4a, 0x6c, 0x5e, 0xf7, 0x40, 0xc1,
                0x68, 0x93, 0x2b, 0x5e, 0x3f, 0x44
            ]
        );

        assert_eq!(
            "5Kd3NBUAdUnhyzenEwVLy9pBKxSwXvE9FMPyR4UKZvpe6E3AgLr"
                .from_mls_b58check(Some(vec![0x80]))
                .unwrap(),
            vec![
                0xed, 0xdb, 0xdc, 0x11, 0x68, 0xf1, 0xda, 0xea, 0xdb, 0xd3, 0xe4, 0x4c, 0x1e, 0x3f,
                0x8f, 0x5a, 0x28, 0x4c, 0x20, 0x29, 0xf7, 0x8a, 0xd2, 0x6a, 0xf9, 0x85, 0x83, 0xa4,
                0x99, 0xde, 0x5b, 0x19
            ]
        );
    }

    #[test]
    fn from_base58check_with_invalid_checksum() {
        assert_eq!(
            "j8YiVRUK8wrJ2wzLH7W6221".from_mls_b58check(Some(vec![TOKEN_ID_PREFIX])),
            Err(FromBase58Error::InvalidChecksum)
        );

        assert_eq!(
            "1PRTTaJesdNovgne6Ehcdu1fpEdX7913C1".from_mls_b58check(Some(vec![0])),
            Err(FromBase58Error::InvalidChecksum)
        );

        assert_eq!(
            "5J3mBbAH58CpQ3Y5RNJpUKPE62SQ5tfcvU2JpbnkeyhfsYB1Jc9"
                .from_mls_b58check(Some(vec![0x80])),
            Err(FromBase58Error::InvalidChecksum)
        );

        assert_eq!(
            "14cxpo3MBCYYWCgF74SWTdcmxipnGUs153".from_mls_b58check(Some(vec![0])),
            Err(FromBase58Error::InvalidChecksum)
        );
        assert_eq!(
            "5JG9hT3beGTJuUAmCQEmNaxAuMacCTfXuw1R3FCXig23RQH1234"
                .from_mls_b58check(Some(vec![0x80])),
            Err(FromBase58Error::InvalidChecksum)
        );

        assert_eq!(
            "1mayif3H2JDC62S4N3rLNtBNRAiUUchek".from_mls_b58check(Some(vec![0])),
            Err(FromBase58Error::InvalidChecksum)
        );
        assert_eq!(
            "5Kd3NBUAdUnhyzenEwVLy9pBKxSwXvE9FMPyR4UKZvpe6E3kehc"
                .from_mls_b58check(Some(vec![0x80])),
            Err(FromBase58Error::InvalidChecksum)
        );
    }

    #[test]
    #[should_panic]
    fn from_base58check_with_invalid_length() {
        "Wh4bh".from_mls_b58check(Some(vec![TOKEN_ID_PREFIX])).unwrap();
    }

    #[test]
    fn base58check_loop() {
        // Using encoding and decoding for 5 times because during these operations the buffer is growing.
        // If we want to have more loops we have to check is it working with more than 128 bytes or not.

        let text = "To be, or not to be";

        let mut buffer = text;
        let mut enc;
        // encode
        for _ in 0..5 {
            enc = buffer.as_bytes().to_mls_b58check(Some(vec![TOKEN_ID_PREFIX]));
            buffer = sp_std::str::from_utf8(enc.as_slice()).unwrap();
        }
        // decode back
        let mut dec;
        for _ in 0..5 {
            dec = buffer.from_mls_b58check(Some(vec![TOKEN_ID_PREFIX])).unwrap();
            buffer = sp_std::str::from_utf8(dec.as_slice()).unwrap();
        }
        assert_eq!(buffer, text);
    }

    #[test]
    fn base58check_bitcoin_test() {
        // Took from bitcoin:
        // https://github.com/bitcoin/bitcoin/blob/master/src/test/base58_tests.cpp
        assert_eq!(
            "3vQB7B6MrGQZaxCuFg4oh".from_mls_b58check(None).unwrap(),
            b"hello world".to_vec()
        );
        assert_eq!(
            "3vQB7B6MrGQZaxCuFg4oi".from_mls_b58check(None),
            Err(FromBase58Error::InvalidChecksum)
        );
        assert_eq!(
            "3vQB7B6MrGQZaxCuFg4oh0IOl".from_mls_b58check(None),
            Err(FromBase58Error::InvalidBase58Character('0', 21))
        );
        assert_eq!(
            "3vQB7B6MrGQZaxCuFg4oh\0".from_mls_b58check(None),
            Err(FromBase58Error::InvalidBase58Character('\0', 21))
        );
    }
}
