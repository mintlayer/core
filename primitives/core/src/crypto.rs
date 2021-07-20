/// Trait suitable for typical cryptographic PKI key public type.
pub trait Public:
    AsRef<[u8]>
    + AsMut<[u8]>
    + Default
    + Derive
    + CryptoType
    + PartialEq
    + Eq
    + Clone
    + Send
    + Sync
    + for<'a> TryFrom<&'a [u8]>
{
    /// A new instance from the given slice.
    ///
    /// NOTE: No checking goes on to ensure this is a real public key. Only use it if
    /// you are certain that the array actually is a pubkey. GIGO!
    fn from_slice(data: &[u8]) -> Self;

    /// Return a `Vec<u8>` filled with raw data.
    fn to_raw_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    /// Return a slice filled with raw data.
    fn as_slice(&self) -> &[u8] {
        self.as_ref()
    }
    /// Return `CryptoTypePublicPair` from public key.
    fn to_public_crypto_pair(&self) -> CryptoTypePublicPair;
}

/// An opaque 32-byte cryptographic identifier.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Default, Encode, Decode, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Hash))]
pub struct AccountId32([u8; 64]);

impl AccountId32 {
    /// Create a new instance from its raw inner byte value.
    ///
    /// Equivalent to this types `From<[u8; 32]>` implementation. For the lack of const
    /// support in traits we have this constructor.
    pub const fn new(inner: [u8; 64]) -> Self {
        Self(inner)
    }
}
