//use hex_literal::hex;

/// Implements standard indexing methods for a given wrapper type
macro_rules! impl_index_newtype {
	($thing:ident, $ty:ty) => {
		impl ::core::ops::Index<usize> for $thing {
			type Output = $ty;

			#[inline]
			fn index(&self, index: usize) -> &$ty {
				&self.0[index]
			}
		}

		impl ::core::ops::Index<::core::ops::Range<usize>> for $thing {
			type Output = [$ty];

			#[inline]
			fn index(&self, index: ::core::ops::Range<usize>) -> &[$ty] {
				&self.0[index]
			}
		}

		impl ::core::ops::Index<::core::ops::RangeTo<usize>> for $thing {
			type Output = [$ty];

			#[inline]
			fn index(&self, index: ::core::ops::RangeTo<usize>) -> &[$ty] {
				&self.0[index]
			}
		}

		impl ::core::ops::Index<::core::ops::RangeFrom<usize>> for $thing {
			type Output = [$ty];

			#[inline]
			fn index(&self, index: ::core::ops::RangeFrom<usize>) -> &[$ty] {
				&self.0[index]
			}
		}

		impl ::core::ops::Index<::core::ops::RangeFull> for $thing {
			type Output = [$ty];

			#[inline]
			fn index(&self, _: ::core::ops::RangeFull) -> &[$ty] {
				&self.0[..]
			}
		}
	};
}

macro_rules! display_from_debug {
	($thing:ident) => {
		impl ::core::fmt::Display for $thing {
			fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(), ::core::fmt::Error> {
				::core::fmt::Debug::fmt(self, f)
			}
		}
	};
}

#[cfg(test)]
macro_rules! hex_script {
	($s:expr) => {
		Script::from(Vec::from(hex!($s)))
	};
}

/// Check given condition and return an error if not satisfied.
///
/// This is useful for exiting a function if given boolean condition is not met. See the example
/// below.
///
/// ```
/// use chainscript::util::check;
///
/// fn div2(x: i32) -> Result<i32, &'static str> {
///     check(x >= 0, "number has to be positive")?;
///     check((x & 1) == 0, "number has to be even")?;
///     Ok(x / 2)
/// }
///
/// assert!(div2(3).is_err());
/// assert!(div2(-5).is_err());
/// assert_eq!(div2(8), Ok(4));
/// ```
pub fn check<E>(c: bool, e: E) -> Result<(), E> {
	c.then(|| ()).ok_or(e)
}

// Export some hash functions.
pub use sp_io::hashing::sha2_256 as sha256;

pub fn sha1(data: &[u8]) -> [u8; 20] {
	use sha1::{Digest, Sha1};
	Sha1::digest(data).into()
}

pub fn ripemd160(data: &[u8]) -> [u8; 20] {
	use ripemd160::{Digest, Ripemd160};
	Ripemd160::digest(data).into()
}

pub fn hash256(data: &[u8]) -> [u8; 32] {
	sha256(&sha256(data))
}

pub fn hash160(data: &[u8]) -> [u8; 20] {
	ripemd160(&sha256(data))
}
