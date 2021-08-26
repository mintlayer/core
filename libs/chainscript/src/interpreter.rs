use crate::{
	context::Context,
	error::Error,
	opcodes,
	script::{self, Instruction, Script},
	util::{self, check},
};
use sp_std::{borrow::Cow, cmp, ops, ops::Range, prelude::*};

/// Item on the data stack.
///
/// The [Cow] type is used to avoid copying data when not necessary. That is often the case with
/// large constants such as public keys and hashes.
type Item<'a> = Cow<'a, [u8]>;

/// Interpreter data stack.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Stack<'a>(Vec<Item<'a>>);

impl<'a> Stack<'a> {
	/// Get stack length.
	fn len(&self) -> usize {
		self.0.len()
	}

	/// Check the stack has at least given number of elements and return the length.
	fn at_least(&self, num: usize) -> crate::Result<usize> {
		(self.len() >= num).then(|| self.len()).ok_or(Error::NotEnoughElementsOnStack)
	}

	/// Pop an item off of the stack.
	fn pop(&mut self) -> crate::Result<Item<'a>> {
		self.0.pop().ok_or(Error::NotEnoughElementsOnStack)
	}

	/// Pop an item of the top of the stack and convert it to bool.
	fn pop_bool(&mut self) -> crate::Result<bool> {
		self.pop().map(|x| script::read_scriptbool(&x))
	}

	/// Pop an item off the stack and convert it to int.
	fn pop_int(&mut self) -> crate::Result<i64> {
		script::read_scriptint(&self.pop()?)
	}

	/// Push an item onto the stack.
	fn push(&mut self, item: Item<'a>) {
		self.0.push(item)
	}

	/// Push a boolean item onto the stack.
	fn push_bool(&mut self, b: bool) {
		self.push_int(b as i64);
	}

	/// Push an integer item onto the stack.
	fn push_int(&mut self, x: i64) {
		//todo!("Check int in the correct range");
		self.push(script::build_scriptint(x).into());
	}

	/// Get an element at given position from the top of the stack.
	pub fn top(&self, idx: usize) -> crate::Result<&Item<'a>> {
		self.at_least(idx + 1).map(|len| &self.0[len - idx - 1])
	}

	/// Map range counting from the top of the stack to the internal vector indexing.
	fn top_range(&self, r: Range<usize>) -> crate::Result<Range<usize>> {
		self.at_least(r.start).map(|len| (len - r.start)..(len - r.end))
	}

	/// Take a slice of the top of the stack.
	fn top_slice(&self, r: Range<usize>) -> crate::Result<&[Item<'a>]> {
		Ok(&self.0[self.top_range(r)?])
	}

	/// Take a mutable slice of the top of the stack.
	fn top_slice_mut(&mut self, r: Range<usize>) -> crate::Result<&mut [Item<'a>]> {
		let i = self.top_range(r)?;
		Ok(&mut self.0[i])
	}

	/// Drop given number of elements
	fn drop(&mut self, num_drop: usize) -> crate::Result<()> {
		let len = self.at_least(num_drop)?;
		Ok(self.0.truncate(len - num_drop))
	}

	/// Duplicate slice indexed from the top of the stack. The new items are added to the top of
	/// the stack.
	fn dup(&mut self, r: Range<usize>) -> crate::Result<()> {
		self.top_range(r).map(|i| self.0.extend_from_within(i))
	}

	/// Swap the top `n` elements with the next `n` elements on the stack.
	fn swap(&mut self, n: usize) -> crate::Result<()> {
		let (top, next) = self.top_slice_mut((2 * n)..0)?.split_at_mut(n);
		Ok(top.swap_with_slice(next))
	}

	/// Remove `n`-th element, counting from the top of the stack.
	fn remove(&mut self, n: usize) -> crate::Result<Item<'a>> {
		let len = self.at_least(n)?;
		Ok(self.0.remove(len - n - 1))
	}
}

/// Execution stack keeps track of masks of IF/ELSE branches being executed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ExecStack {
	stack: Vec<bool>,
	num_idle: usize,
}

impl ExecStack {
	/// Push mask onto the stack. Executing: true, not executing: false.
	fn push(&mut self, executing: bool) {
		self.stack.push(executing);
		self.num_idle += (!executing) as usize;
	}

	/// Pop the top item off the stack.
	fn pop(&mut self) -> Option<bool> {
		let executing = self.stack.pop()?;
		self.num_idle -= (!executing) as usize;
		Some(executing)
	}

	/// Check if we are currently executing, i.e. no branch is masked out.
	fn executing(&self) -> bool {
		self.num_idle == 0
	}

	/// Check the execution stack is empty, i.e. we are not inside of a conditional.
	fn is_empty(&self) -> bool {
		self.stack.is_empty()
	}
}

/// Run given script with given initial stack.
///
/// Consumes the stack. Returns either an error or the final stack.
pub fn run_interpreter<'a, Ctx: Context>(
	ctx: &Ctx,
	script: &'a Script,
	mut stack: Stack<'a>,
) -> crate::Result<Stack<'a>> {
	if script.len() > Ctx::MAX_SCRIPT_SIZE {
		return Err(Error::ScriptSize)
	}

	let mut instr_iter = script.instructions_iter(ctx.enforce_minimal_push());
	let mut subscript: &[u8] = instr_iter.subscript();
	let mut exec_stack = ExecStack::default();
	let mut alt_stack = Stack::<'a>::default();

	while let Some(instr) = instr_iter.next() {
		let instr = instr?;

		let executing = exec_stack.executing();
		match instr {
			Instruction::PushBytes(data) => {
				check(data.len() <= Ctx::MAX_SCRIPT_ELEMENT_SIZE, Error::PushSize)?;
				if executing {
					stack.push(data.into());
				}
			},
			Instruction::Op(opcode) => match opcode.classify() {
				opcodes::Class::NoOp => (),
				opcodes::Class::IllegalOp => return Err(Error::IllegalOp),
				opcodes::Class::ReturnOp if executing => return Err(Error::VerifyFail),
				opcodes::Class::PushNum(x) if executing => stack.push_int(x as i64),
				opcodes::Class::PushBytes(_) =>
					unreachable!("Already handled using Instruction::PushBytes"),
				opcodes::Class::PushData(_) =>
					unreachable!("Already handled using Instruction::PushBytes"),
				opcodes::Class::AltStack(opc) => match opc {
					opcodes::AltStack::OP_TOALTSTACK => alt_stack.push(stack.pop()?),
					opcodes::AltStack::OP_FROMALTSTACK => stack.push(alt_stack.pop()?),
				},
				opcodes::Class::Signature(sig_opcode) => {
					match sig_opcode {
						opcodes::Signature::OP_CODESEPARATOR => subscript = instr_iter.subscript(),
						opcodes::Signature::OP_CHECKSIG | opcodes::Signature::OP_CHECKSIGVERIFY => {
							let pubkey = stack.pop()?;
							let sig = stack.pop()?;
							// Treat plain CHECKSIG as 1-of-1 MULTISIG
							let result = check_multisig(
								ctx,
								core::iter::once(sig.as_ref()),
								core::iter::once(pubkey.as_ref()),
								subscript,
							)?;
							stack.push_bool(result);
						},
						opcodes::Signature::OP_CHECKMULTISIG |
						opcodes::Signature::OP_CHECKMULTISIGVERIFY => {
							// Extract keys
							let nkey = stack.pop_int()?;
							check(nkey >= 0, Error::PubkeyCount)?;
							let nkey = nkey as usize;
							check(nkey <= Ctx::MAX_PUBKEYS_PER_MULTISIG, Error::PubkeyCount)?;
							let keys = stack.top_slice(nkey..0)?.iter().map(AsRef::as_ref);

							// Extract signatures
							let nsig = script::read_scriptint(stack.top(nkey)?)?;
							check(nsig >= 0, Error::SigCount)?;
							let nsig = nsig as usize;
							check(nsig <= nkey, Error::SigCount)?;
							let sig_range = (nsig + nkey + 1)..(nkey + 1);
							let sigs = stack.top_slice(sig_range)?.iter().map(AsRef::as_ref);

							// Verify
							let result = check_multisig(ctx, sigs, keys, subscript)?;

							// Clean up stack, check dummy 0.
							stack.drop(nsig + nkey + 1)?;
							if !stack.pop()?.is_empty() {
								return Err(Error::NullDummy)
							}
							stack.push_bool(result);
						},
					}
					if sig_opcode.is_verify() && !stack.pop_bool()? {
						return Err(Error::VerifyFail)
					}
				},
				opcodes::Class::ControlFlow(cf) => match cf {
					opcodes::ControlFlow::OP_IF | opcodes::ControlFlow::OP_NOTIF => {
						let cond = executing && {
							let cond = match stack.pop()?.as_ref() {
								c if !ctx.enforce_minimal_if() => script::read_scriptbool(c),
								&[] => false,
								&[1u8] => true,
								_ => return Err(Error::InvalidOperand),
							};
							cond ^ (cf == opcodes::ControlFlow::OP_NOTIF)
						};
						exec_stack.push(cond);
					},
					opcodes::ControlFlow::OP_ELSE => {
						let top_executing = exec_stack.pop().ok_or(Error::UnbalancedIfElse)?;
						exec_stack.push(!top_executing);
					},
					opcodes::ControlFlow::OP_ENDIF => {
						let _ = exec_stack.pop().ok_or(Error::UnbalancedIfElse)?;
					},
				},
				opcodes::Class::Ordinary(opcode) if executing => {
					execute_opcode(opcode, &mut stack)?;
				},
				_ => (),
			},
		}
	}

	// Check OP_IF/OP_NOTIF has been closed properly wiht OP_ENDIF.
	if !exec_stack.is_empty() {
		return Err(Error::UnbalancedIfElse)
	}

	Ok(stack)
}

/// Check whether given signatures are valid for given pubkeys.
/// Some pubkeys may not have signatures to go with them.
fn check_multisig<'a, Ctx: Context>(
	ctx: &Ctx,
	mut sigs: impl Iterator<Item = &'a [u8]> + ExactSizeIterator,
	mut pubkeys: impl Iterator<Item = &'a [u8]> + ExactSizeIterator,
	subscript: &[u8],
) -> crate::Result<bool> {
	// Check each signature has its corresponding pubkey.
	while let Some(sig) = sigs.next() {
		let sig = ctx.parse_signature(sig).ok_or(Error::SignatureFormat)?;
		let txhash = ctx.hash_transaction(&sig, subscript);
		// Find pubkey matching this signature.
		loop {
			if pubkeys.len() < sigs.len() + 1 {
				// Not enough pubkeys to cover all the remaining signatures.
				// We add 1 to the number of signatures left in the condition to account for the
				// signature being processed that has just been taken out of the iterator.
				return Ok(false)
			}
			let pubkey = ctx.parse_pubkey(pubkeys.next().unwrap()).ok_or(Error::PubkeyFormat)?;
			if ctx.verify_signature(&sig, &pubkey, &txhash) {
				break
			}
		}
	}
	Ok(true)
}

/// Execute an ["ordinay"](opcode::Ordinary) opcode.
fn execute_opcode<'a>(opcode: opcodes::Ordinary, stack: &mut Stack<'a>) -> crate::Result<()> {
	use opcodes::Ordinary as Opc;

	match opcode {
		// Verify. Do nothing now, the actual verification is handled below this match statement.
		Opc::OP_VERIFY => (),

		// Main stack manipulation
		Opc::OP_DROP => stack.drop(1)?,
		Opc::OP_2DROP => stack.drop(2)?,
		Opc::OP_DUP => stack.dup(1..0)?,
		Opc::OP_2DUP => stack.dup(2..0)?,
		Opc::OP_3DUP => stack.dup(3..0)?,
		Opc::OP_OVER => stack.dup(2..1)?,
		Opc::OP_2OVER => stack.dup(4..2)?,
		Opc::OP_SWAP => stack.swap(1)?,
		Opc::OP_2SWAP => stack.swap(2)?,
		Opc::OP_2ROT => {
			let top = stack.top_slice_mut(0..6)?;
			let to_put = [2, 3, 4, 5, 0, 1].map(|i| top[i].clone());
			top.clone_from_slice(&to_put);
		},
		Opc::OP_NIP => {
			let x = stack.pop()?;
			let _ = stack.pop()?;
			stack.push(x);
		},
		Opc::OP_PICK => {
			let i = stack.pop_int()?;
			check(i >= 0, Error::InvalidOperand)?;
			stack.push(stack.top(i as usize)?.clone());
		},
		Opc::OP_ROLL => {
			let i = stack.pop_int()?;
			check(i >= 0, Error::InvalidOperand)?;
			let x = stack.remove(i as usize)?;
			stack.push(x);
		},
		Opc::OP_ROT => {
			let x = stack.remove(2)?;
			stack.push(x);
		},
		Opc::OP_TUCK => {
			let x = stack.top(0)?.clone();
			stack.swap(1)?;
			stack.push(x);
		},
		Opc::OP_IFDUP => {
			let item = stack.top(0)?;
			if script::read_scriptbool(item) {
				let item_clone = item.clone();
				stack.push(item_clone);
			}
		},
		Opc::OP_DEPTH => {
			check(stack.len() < i32::MAX as usize, Error::NumericOverflow)?;
			stack.push_int(stack.len() as i64);
		},

		// Stack item queries
		Opc::OP_SIZE => {
			let top_len = stack.top(0)?.len();
			check(top_len < i32::MAX as usize, Error::NumericOverflow)?;
			stack.push_int(top_len as i64);
		},
		Opc::OP_EQUAL | Opc::OP_EQUALVERIFY => {
			let y = stack.pop()?;
			let x = stack.pop()?;
			stack.push_bool(x == y);
		},

		// Arithmetic
		Opc::OP_1ADD => op_num1(stack, |x| x + 1)?,
		Opc::OP_1SUB => op_num1(stack, |x| x - 1)?,
		Opc::OP_NEGATE => op_num1(stack, ops::Neg::neg)?,
		Opc::OP_ABS => op_num1(stack, i64::abs)?,
		Opc::OP_NOT => op_num1(stack, |x| (x == 0) as i64)?,
		Opc::OP_0NOTEQUAL => op_num1(stack, |x| (x != 0) as i64)?,
		Opc::OP_ADD => op_num2(stack, ops::Add::add)?,
		Opc::OP_SUB => op_num2(stack, ops::Sub::sub)?,
		Opc::OP_BOOLAND => op_num2(stack, |x, y| (x != 0 && y != 0) as i64)?,
		Opc::OP_BOOLOR => op_num2(stack, |x, y| (x != 0 || y != 0) as i64)?,
		Opc::OP_NUMEQUAL | Opc::OP_NUMEQUALVERIFY => op_num2(stack, |x, y| (x == y) as i64)?,
		Opc::OP_NUMNOTEQUAL => op_num2(stack, |x, y| (x != y) as i64)?,
		Opc::OP_LESSTHAN => op_num2(stack, |x, y| (x < y) as i64)?,
		Opc::OP_GREATERTHAN => op_num2(stack, |x, y| (x > y) as i64)?,
		Opc::OP_LESSTHANOREQUAL => op_num2(stack, |x, y| (x <= y) as i64)?,
		Opc::OP_GREATERTHANOREQUAL => op_num2(stack, |x, y| (x >= y) as i64)?,
		Opc::OP_MIN => op_num2(stack, cmp::min)?,
		Opc::OP_MAX => op_num2(stack, cmp::max)?,
		Opc::OP_WITHIN => {
			let hi = stack.pop_int()?;
			let lo = stack.pop_int()?;
			let x = stack.pop_int()?;
			stack.push_int((lo <= x && x < hi) as i64);
		},

		// Hashes
		Opc::OP_RIPEMD160 => op_hash(stack, util::ripemd160)?,
		Opc::OP_SHA1 => op_hash(stack, util::sha1)?,
		Opc::OP_SHA256 => op_hash(stack, util::sha256)?,
		Opc::OP_HASH160 => op_hash(stack, util::hash160)?,
		Opc::OP_HASH256 => op_hash(stack, util::hash256)?,
	}

	if opcode.is_verify() && !stack.pop_bool()? {
		Err(Error::VerifyFail)
	} else {
		Ok(())
	}
}

/// Perform an unary arithmetic operation on the top of the stack.
fn op_num1(stack: &mut Stack, f: impl FnOnce(i64) -> i64) -> crate::Result<()> {
	let x = stack.pop_int()?;
	Ok(stack.push_int(f(x)))
}

/// Perform a binary arithmetic operation on the top of the stack.
fn op_num2(stack: &mut Stack, f: impl FnOnce(i64, i64) -> i64) -> crate::Result<()> {
	let y = stack.pop_int()?;
	let x = stack.pop_int()?;
	Ok(stack.push_int(f(x, y)))
}

/// Perform a byte-array based function on the top stack item. Useful for hashes.
fn op_hash<T: AsRef<[u8]>>(stack: &mut Stack, f: impl FnOnce(&[u8]) -> T) -> crate::Result<()> {
	let result = f(&stack.pop()?);
	Ok(stack.push(Cow::Owned(result.as_ref().to_vec())))
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{context::testcontext::TestContext, script::Builder};
	use hex_literal::hex;
	use proptest::{collection::SizeRange, prelude::*};

	#[test]
	fn unit_exec_stack() {
		let mut exec_stack = ExecStack::default();
		assert!(exec_stack.executing());

		exec_stack.push(true);
		assert!(exec_stack.executing());
		exec_stack.push(false);
		assert!(!exec_stack.executing());
		exec_stack.push(true);
		assert!(!exec_stack.executing());

		// [true, false, true]
		let _ = exec_stack.pop();
		// [true, false]
		assert!(!exec_stack.executing());
		let _ = exec_stack.pop();
		// [true]
		assert!(exec_stack.executing());
		let _ = exec_stack.pop();
		// []
		assert!(exec_stack.executing());

		assert!(exec_stack.stack.is_empty());
	}

	#[test]
	fn unit_if_then_else_syntax() {
		let should_fail = |script: Script| {
			let stack = Stack(vec![vec![].into(), vec![].into()]);
			let ctx = TestContext::default();
			let result = run_interpreter(&ctx, &script, stack);
			assert_eq!(result, Err(Error::UnbalancedIfElse));
		};
		use opcodes::all::*;
		should_fail(Builder::new().push_opcode(OP_IF).into_script());
		should_fail(Builder::new().push_opcode(OP_IF).push_opcode(OP_ELSE).into_script());
		should_fail(Builder::new().push_opcode(OP_IF).push_opcode(OP_NOTIF).into_script());
		should_fail(Builder::new().push_opcode(OP_ELSE).into_script());
		should_fail(Builder::new().push_opcode(OP_ENDIF).into_script());
		should_fail(
			Builder::new()
				.push_opcode(OP_IF)
				.push_opcode(OP_IF)
				.push_opcode(OP_ELSE)
				.push_opcode(OP_ENDIF)
				.into_script(),
		);
		should_fail(
			Builder::new()
				.push_opcode(OP_IF)
				.push_opcode(OP_ELSE)
				.push_opcode(OP_ENDIF)
				.push_opcode(OP_ENDIF)
				.into_script(),
		);
	}

	#[test]
	fn unit_multisig() {
		let ctx = TestContext::default();
		let txhash = ctx.hash_transaction(&[0u8; 4], &[]);
		let sign_by = |pk: [u8; 4]| -> [u8; 4] { [0, 1, 2, 3].map(|i| pk[i] ^ txhash[i]) };

		let keys = [hex!("01010101"), hex!("02020202"), hex!("03030303"), hex!("04040404")];
		let sig0 = sign_by(hex!("00000000"));
		let sig1 = sign_by(hex!("01010101"));
		let sig2 = sign_by(hex!("02020202"));
		let sig3 = sign_by(hex!("03030303"));
		let sig4 = sign_by(hex!("04040404"));

		let test_case = |sigs: &[[u8; 4]], expected: crate::Result<bool>| {
			let result = check_multisig(
				&ctx,
				sigs.iter().map(AsRef::as_ref),
				keys.iter().map(AsRef::as_ref),
				&[],
			);
			assert_eq!(result, expected);
		};

		// sig0 is not in the set of public keys, this should fail
		test_case(&[sig1, sig0][..], Ok(false));
		// This should be fine, sig1 and sig2 in the correct order
		test_case(&[sig1, sig2][..], Ok(true));
		// Try first and last
		test_case(&[sig1, sig4][..], Ok(true));
		// And last two
		test_case(&[sig3, sig4][..], Ok(true));
		// This should fail due to incorrect signature order
		test_case(&[sig2, sig1][..], Ok(false));
		// Now with all the signatures.
		test_case(&[sig1, sig2, sig3, sig4][..], Ok(true));
		// Duplicate signature should not count as two sigs.
		test_case(&[sig1, sig1][..], Ok(false));
	}

	// Generate stack item as an array of bytes
	fn gen_item_bytes<'a>(num_bytes: Range<usize>) -> impl Strategy<Value = Item<'a>> {
		prop::collection::vec(prop::num::u8::ANY, num_bytes).prop_map(|v| v.into())
	}

	// Generate stack with given item generation strategy.
	fn gen_stack<'a>(
		gen_item: impl Strategy<Value = Item<'a>>,
		size: impl Into<SizeRange>,
	) -> impl Strategy<Value = Stack<'a>> {
		prop::collection::vec(gen_item, size).prop_map(Stack)
	}

	proptest! {
		#[test]
		fn prop_exec_stack_push(items in prop::collection::vec(prop::bool::ANY, 0..9)) {
			let mut exec_stack = ExecStack::default();
			items.iter().for_each(|i| exec_stack.push(*i));

			// Check the final state of the execution stack
			assert_eq!(items, exec_stack.stack);
			// Check number of idle lanes is correct
			assert_eq!(items.iter().filter(|i| !**i).count(), exec_stack.num_idle);
			// Check whether executing indicator is correct
			assert_eq!(items.iter().all(|i| *i), exec_stack.executing());
		}

		#[test]
		fn prop_exec_stack_push_pop(items0 in prop::collection::vec(prop::bool::ANY, 0..9),
									items1 in prop::collection::vec(prop::bool::ANY, 1..9)) {
			let mut exec_stack = ExecStack::default();
			items0.iter().for_each(|i| exec_stack.push(*i));
			let orig_exec_stack = exec_stack.clone();

			// Push a bunch of extra items and pop them again
			items1.iter().for_each(|i| exec_stack.push(*i));
			items1.iter().for_each(|_| exec_stack.pop().map(|_| ()).unwrap());

			// Check we got to the original state
			assert_eq!(orig_exec_stack, exec_stack);
		}

		#[test]
		fn prop_2dup(mut stack in gen_stack(gen_item_bytes(0..40), 2..10)) {
			let res = execute_opcode(opcodes::Ordinary::OP_2DUP, &mut stack);
			prop_assert!(res.is_ok());
			prop_assert_eq!(stack.top(0), stack.top(2));
			prop_assert_eq!(stack.top(1), stack.top(3));
		}

		#[test]
		fn prop_swap_swap(orig_stack in gen_stack(gen_item_bytes(0..40), 2..5)) {
			let mut stack = orig_stack.clone();
			execute_opcode(opcodes::Ordinary::OP_SWAP, &mut stack).unwrap();
			execute_opcode(opcodes::Ordinary::OP_SWAP, &mut stack).unwrap();
			prop_assert_eq!(orig_stack.0, stack.0);
		}

		#[test]
		fn prop_if_then(cond: bool, then_val: i32, else_val: i32) {
			let script = Builder::new()
				.push_int(cond as i64)
				.push_opcode(opcodes::all::OP_IF)
				.push_int(then_val as i64)
				.push_opcode(opcodes::all::OP_ELSE)
				.push_int(else_val as i64)
				.push_opcode(opcodes::all::OP_ENDIF)
				.into_script();

			let stack = Stack::default();
			let ctx = TestContext::default();
			let result = run_interpreter(&ctx, &script, stack);
			prop_assert!(result.is_ok());

			let expected = cond.then(|| then_val).unwrap_or(else_val) as i64;
			let expected_stack = Stack(vec![script::build_scriptint(expected).into()]);
			prop_assert_eq!(result.unwrap(), expected_stack);
		}
	}
}
