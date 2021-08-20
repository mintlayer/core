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
