use bech32_no_std::{self, u5, Error, FromBase32, ToBase32};
use codec::{Decode, Encode};
use core::fmt;
use sp_std::vec::Vec;

/// A multi-format address wrapper for on-chain accounts.
#[derive(Encode, Decode, PartialEq, Eq, Clone, crate::RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum MultiAddress<AccountId, AccountIndex> {
    /// It's an account ID (pubkey).
    Id(AccountId),
    /// It's an account index.
    Index(#[codec(compact)] AccountIndex),
    /// It's some arbitrary raw bytes.
    Raw(Vec<u8>),
    /// It's a 32 byte representation.
    Bech32([u8; 32]),
}

// I have some doubt about which format in MultiAddress<AccountId, AccountIndex>.inner and at the moment I'm digging it
fn multiaddress_as_bech32(data: &[u8; 32]) -> &str {
    unimplemented!();
}

// For map_err fnOnce that could transform an error bech32_no_std
fn fmt_err_fn(_x: bech32_no_std::Error) -> sp_std::fmt::Error {
    fmt::Error
}

#[cfg(feature = "std")]
impl<AccountId, AccountIndex> std::fmt::Display for MultiAddress<AccountId, AccountIndex>
where
    AccountId: std::fmt::Debug,
    AccountIndex: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use sp_core::hexdisplay::HexDisplay;
        match self {
            Self::Raw(inner) => write!(f, "MultiAddress::Raw({})", HexDisplay::from(inner)),
            Self::Bech32(inner) => {
                let (_, data) =
                    bech32_no_std::decode(multiaddress_as_bech32(inner)).map_err(fmt_err_fn)?;
                write!(
                    f,
                    "MultiAddress::Bech32({})",
                    HexDisplay::from(&data.into_iter().map(|x| x.to_u8()).collect::<Vec<u8>>())
                )
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

impl<AccountId, AccountIndex> From<AccountId> for MultiAddress<AccountId, AccountIndex> {
    fn from(a: AccountId) -> Self {
        Self::Id(a)
    }
}

impl<AccountId: Default, AccountIndex> Default for MultiAddress<AccountId, AccountIndex> {
    fn default() -> Self {
        Self::Id(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use bech32_no_std::u5;
    use bech32_no_std::{self}; //, FromBase32, ToBase32};
    use bitcoin_hashes::{ripemd160, sha256};
    //use std::convert::TryFrom;
    //use crate::{crypto::{DEV_PHRASE, set_default_ss58_version}, keccak_256};
    pub const DEV_PHRASE: &str =
        "bottom drive obey lake curtain smoke basket hold race lonely fit walk";

    #[test]
    fn make_an_account() {
        let mnemonic = "sample split bamboo west visual approve brain fox arch impact relief smile";
    }

    #[test]
    fn bech32_valid_address() {
        let _compressed_privkey = "L58kXqwx8JUWoVm4EuaX9bFeCSYWcwiuTKCFvxoFsE4p7GoorRDC";
        let bech32_p2wpkh = "bc1q2wzdwh9znl8jz306ncgagapmaevkqt68g25klg";
        let _bech32_p2wsh = "bc1qnrsum0njvrk92rm4kf46a2rv5yqwgccxgg4vkqv9pwczhz5wtltszfqyuy";
        let _bech32_sha256 = "A51066EDD669F9BC2400361A6DFAC289C91E359AAC144CA30C3A27387D695603";
        let _compressed_pubkey: Vec<u8> = vec![
            0x03, 0x18, 0x26, 0xD3, 0xED, 0xB1, 0xAE, 0x8E, 0x7E, 0xB7, 0xDB, 0xF0, 0xF1, 0x44,
            0xE1, 0xFF, 0x3F, 0x39, 0xBD, 0x5B, 0x8D, 0xA2, 0x57, 0x3C, 0xEB, 0x8A, 0xA0, 0x1E,
            0x91, 0x86, 0x90, 0xBD, 0x8F,
        ];
        let data_u5: Vec<u5> = vec![
            0, /* Padding */
            10, 14, 02, 13, 14, 23, 05, 02, 19, 31, 07, 18, 02, 17, 15, 26, 19, 24, 08, 29, 08, 29,
            01, 27, 29, 25, 12, 22, 00, 11, 26, 07,
        ]
        .into_iter()
        .map(|x| u5::try_from_u8(x).unwrap())
        .collect();

        let (hrp, data) = bech32_no_std::decode(&bech32_p2wpkh).unwrap();
        dbg!(&data
            .clone()
            .into_iter()
            .map(|x| x.to_u8())
            .collect::<Vec<u8>>());

        assert_eq!(hrp, "bc");
        assert_eq!(&data, &data_u5);
    }

    #[test]
    fn bech32_pubkey_to_addr() {
        // There are many steps involved in it.
        // hash160(publickey) which is ripemd160(sha256(publickey)).
        // After that add 0 Uint8 to the output of bech32 words.
        // Then using bech32 encode it with the prefix bc for bitcoin.

        let _compressed_pubkey: Vec<u8> = vec![
            0x03, 0x18, 0x26, 0xD3, 0xED, 0xB1, 0xAE, 0x8E, 0x7E, 0xB7, 0xDB, 0xF0, 0xF1, 0x44,
            0xE1, 0xFF, 0x3F, 0x39, 0xBD, 0x5B, 0x8D, 0xA2, 0x57, 0x3C, 0xEB, 0x8A, 0xA0, 0x1E,
            0x91, 0x86, 0x90, 0xBD, 0x8F,
        ];

        /*
        println!("{}", _compressed_pubkey.len());

        let h = sha256::Hash::hash(&[]);
        let mut engine = HashEngine::default();
        let mut engine = sha256::Hash(<[u8; 32]>::try_from(&_compressed_pubkey[..]).unwrap());
        //engine.

         */
    }

    #[test]
    fn bech32_soft_known_pair_should_work() {
        let pair = Pair::from_string(&format!("{}/Alice", DEV_PHRASE), None).unwrap();
        // known address of DEV_PHRASE with 1.1
        //let known =
        //    hex_literal::hex!("d6c71059dbbe9ad2b0ed3f289738b800836eb425544ce694825285b958ca755e");
        //assert_eq!(pair.public().to_raw_vec(), known);

        let pub_key: Vec<u8> = pair.public().to_raw_vec();
        dbg!(pub_key);
    }
}
