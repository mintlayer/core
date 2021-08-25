use displaydoc::Display;

/// Ways that a script might fail. Not everything is split up as
/// much as it could be; patches welcome if more detailed errors
/// would help you.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy, Display)]
pub enum Error {
	/// Something did a non-minimal push
	NonMinimalPush,
	/// Some opcode expected a parameter, but it was missing or truncated
	EarlyEndOfScript,
	/// Tried to read an array off the stack as a number when it was more than 4 bytes
	NumericOverflow,
	/// Illegal instruction executed
	IllegalOp,
	/// Syntactically incorrect OP_(NOT)IF/OP_ELSE/OP_ENDIF
	UnbalancedIfElse,
	/// Stack has insufficient number of elements in it
	NotEnoughElementsOnStack,
	/// Invalid operand to a script operation.
	InvalidOperand,
	/// OP_*VERIFY failed verification or OP_RETURN was executed.
	VerifyFail,
	/// Signature is not in correct format.
	SignatureFormat,
	/// Pubkey is not in correct format.
	PubkeyFormat,
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
