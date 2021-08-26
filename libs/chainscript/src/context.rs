/// Context for the script interpreter.
///
/// This trait defines how the interpreter interfaces with the blockchain. It allows the client
/// to adjust the behaviour of the interpreter by having pluggable signature verification routines
/// and by selectively enabling various interpreter features. It is expected that all data the
/// interpreter takes as implicit inputs (such as transaction hashes that the signatures sign,
/// block height for time locks, etc.) are provided by a type that implements Context.
///
/// ## TODO
///
/// * This interface is currently rather monolithic. It may be sensible to break it down into
///   multiple smaller ones for greater flexibility in the future.
/// * The interface is not fully finalized. It is likely to change as new requirements come in. E.g.
///   it may be currently too limited to support signature batching.
pub trait Context {
	/// Maximum number of bytes pushable to the stack
	const MAX_SCRIPT_ELEMENT_SIZE: usize = 520;

	/// Maximum number of public keys per multisig
	const MAX_PUBKEYS_PER_MULTISIG: usize;

	/// Maximum script length in bytes
	const MAX_SCRIPT_SIZE: usize;

	/// Signature, parsed and verified for correct format.
	type Signature;

	/// Public key type.
	type Public;

	/// Transaction hash type used by the chain for signatures.
	type TxHash: AsRef<[u8]>;

	/// Extract a signature and check it is in the correct format.
	fn parse_signature(&self, sig: &[u8]) -> Option<Self::Signature>;

	/// Extract a pubkey and check it is in the correct format.
	fn parse_pubkey(&self, pk: &[u8]) -> Option<Self::Public>;

	/// Hash the transaction.
	///
	/// Signature is passed in as well to give the method an opportunity to extract sighash and
	/// decide which parts of the transaction should contribute to the hash.
	fn hash_transaction(&self, signature: &Self::Signature, subscript: &[u8]) -> Self::TxHash;

	/// Verify signature.
	fn verify_signature(
		&self,
		sig: &Self::Signature,
		pk: &Self::Public,
		msg: &Self::TxHash,
	) -> bool;

	/// Check absolute time lock.
	fn check_lock_time(&self, _lock_time: i64) -> bool {
		false
	}

	/// Check relative time lock.
	fn check_sequence(&self, _sequence: i64) -> bool {
		false
	}

	/// Enforce minimal push.
	fn enforce_minimal_push(&self) -> bool {
		true
	}

	/// Force the condition for OP_(NOT)IF to be either [] or [0x01], fail script otherwise.
	fn enforce_minimal_if(&self) -> bool {
		true
	}
}

// A test context implementation.
// Used for testing and as an example of what a Context might look like.
#[cfg(any(test, feature = "testcontext"))]
pub mod testcontext {

	use super::*;
	use crate::util::sha256;
	use core::convert::TryFrom;

	#[derive(Default)]
	pub struct TestContext {
		transaction: Vec<u8>,
	}

	/// Test context.
	///
	/// The Context implementation for testing. The transaction hash (just 4 bytes for tesing) has
	/// to be provided explicitly as a byte string. Signature scheme is very simple: The bitwise xor
	/// of transaction hash, signature and public key has to be equal to zero. Not recommended for
	/// production.
	impl TestContext {
		pub fn new(transaction: Vec<u8>) -> Self {
			Self { transaction }
		}
	}

	impl Context for TestContext {
		const MAX_PUBKEYS_PER_MULTISIG: usize = 20;
		const MAX_SCRIPT_SIZE: usize = 10000;

		// Signatures, keys and transaction IDs are just 4-byte binary data each.
		type Signature = [u8; 4];
		type Public = [u8; 4];
		type TxHash = [u8; 4];

		fn parse_signature(&self, sig: &[u8]) -> Option<Self::Signature> {
			Self::Signature::try_from(sig).ok()
		}

		fn parse_pubkey(&self, pk: &[u8]) -> Option<Self::Public> {
			Self::Public::try_from(pk).ok()
		}

		fn hash_transaction(&self, _sig: &Self::Signature, subscript: &[u8]) -> Self::TxHash {
			let data = [&self.transaction[..], subscript].concat();
			Self::TxHash::try_from(&sha256(&data)[0..4]).unwrap()
		}

		fn verify_signature(
			&self,
			sig: &Self::Signature,
			pk: &Self::Public,
			msg: &Self::TxHash,
		) -> bool {
			sig.iter().zip(pk.iter()).zip(msg.iter()).all(|((&s, &p), &m)| (s ^ p ^ m) == 0)
		}
	}
}
