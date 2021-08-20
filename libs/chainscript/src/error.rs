use core::fmt;

/// Ways that a script might fail. Not everything is split up as
/// much as it could be; patches welcome if more detailed errors
/// would help you.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
pub enum Error {
	/// Something did a non-minimal push; for more information see
	/// `https://github.com/bitcoin/bips/blob/master/bip-0062.mediawiki#Push_operators`
	NonMinimalPush,
	/// Some opcode expected a parameter, but it was missing or truncated
	EarlyEndOfScript,
	/// Tried to read an array off the stack as a number when it was more than 4 bytes
	NumericOverflow,
	/// Illegal instruction executed
	IllegalOp,
	/// Syntactically inforrect OP_(NOT)IF/OP_ELSE/OP_ENDIF
	UnbalancedIfElse,
	/// Stack has insufficient number of elements in it
	NotEnoughElementsOnStack,
	/// Invalid operand to a script operation.
	InvalidOperand,
	#[cfg(feature = "bitcoinconsensus")]
	/// Error validating the script with bitcoinconsensus library
	BitcoinConsensus(bitcoinconsensus::Error),
	#[cfg(feature = "bitcoinconsensus")]
	/// Can not find the spent output
	UnknownSpentOutput(OutPoint),
	#[cfg(feature = "bitcoinconsensus")]
	/// Can not serialize the spending transaction
	SerializationError,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let str = match *self {
			Error::NonMinimalPush => "non-minimal datapush",
			Error::EarlyEndOfScript => "unexpected end of script",
			Error::NumericOverflow => "numeric overflow (number on stack larger than 4 bytes)",
			Error::IllegalOp => "illegal script operation executed",
			Error::UnbalancedIfElse => "OP_IF/OP_NOTIF/OP_ELSE/OP_ENDIF not syntactically correct",
			Error::NotEnoughElementsOnStack => "stack does not have enough elements",
			Error::InvalidOperand => "invalid operand to a script operation",
			#[cfg(feature = "bitcoinconsensus")]
			Error::BitcoinConsensus(ref _n) => "bitcoinconsensus verification failed",
			#[cfg(feature = "bitcoinconsensus")]
			Error::UnknownSpentOutput(ref _point) => "unknown spent output Transaction::verify()",
			#[cfg(feature = "bitcoinconsensus")]
			Error::SerializationError =>
				"can not serialize the spending transaction in Transaction::verify()",
		};
		f.write_str(str)
	}
}

#[cfg(feature = "std")]
impl ::std::error::Error for Error {}

#[cfg(feature = "bitcoinconsensus")]
#[doc(hidden)]
impl From<bitcoinconsensus::Error> for Error {
	fn from(err: bitcoinconsensus::Error) -> Error {
		match err {
			_ => Error::BitcoinConsensus(err),
		}
	}
}

pub type Result<T> = core::result::Result<T, Error>;
