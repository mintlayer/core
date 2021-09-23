# Signing Transactions

This document defines the serialization format used to obtain message used to sign transactions or
verify transaction signatures in Mintlayer's UTXO model.

## Definitions and notation

Constants `SIGHASH_*` have the same values as in Bitcoin

`hash(x)`: a 256-bit BLAKE2 hash of the SCALE serialization of `x`

`seq[i]` gives `i`-th element of a sequence

`x.field` access a struct field

`seq.*field`: given a vector `seq`, this is equivalent to Python pseudo-code `[ x.field for x in seq ]`

## Pubkey Format

The first byte determines signature scheme used for this public key.
The key itself immediately follows.

* If signature type is:
  * Schnorr:
    * [1B] constant `0x00`
    * [32B] Schnorr pubkey

## Signature Format

Different signature schemes have different signature sizes.

* [`N` Bytes] signature
  * The length `N` is determined by corresponding pubkey type
* If `sighash` is:
  * 0x00 (default):
    * [0B] nothing
  * otherwise:
    * [1B] sighash

## Message Format

Given:

* `tx`: transaction
* `index`: the index of input curretnly under consideration
* `sighash`: signature mode (1 byte)
* `input_utxos`: a sequence of UTXOs being spent by transaction inputs
  * `index` < `input_utxos.len()` = `tx.inputs.len()`
* `codesep_pos`: 4-byte position of the last `OP_CODESEPARATOR`
  * position is in number of decoded instructions (executed or not)
  * or `0xffffffff` if no `OP_CODESEPARATOR` has been seen so far
    or we are outside of a script context (e.g. plain pay to pubkey, no script involved)

The message is a concatenation of:

* [1B] sighash
* If `sighash & SIGHASH_ANYONECANPAY` is:
  * 0:
    * [1B] constant `0x00`
    * [32B] outpoints hash `hash(tx.inputs.*outpoint)`
    * [32B] hash of UTXOs being spent `hash(input_utxos)`
    * [8B] `index`
  * non-0:
    * [1B] constant `0x01`
    * [32B] current outpoint `tx.inputs[index].outpoint`
    * [32B] hash of UTXO being spent `hash(input_utxos[index])`
* If `sighash & 0x03` is:
  * `SIGHASH_ALL`:
    * [1B] constant `0x01`
    * [32B] hash of all outputs `hash(tx.outputs)`
  * `SIGHASH_NONE`:
    * [1B] constant `0x02`
  * `SIGHASH_SINGLE`
    * [1B] constant `0x03`
    * If `index` < `tx.outputs.len()`:
      * true:
        * [32B] hash of output matching current input index `hash(tx.outputs[index])`
      * false:
        * [32B] constant hash `0x0000...00000`
* [4B] `codesep_pos`

## Notes

The `tx.inputs.*witness` fields are not signed since they cointain the signatures.

The `tx.inputs.*lock` fields do not show up directly. They are, however, committed to indirectly
because the `lock` field is fully determined by contents of the output being spent by the input
by requiring it to conform to a particular hash (e.g. like in P2SH or requiring it to be empty).

Many of the hashes included in the resultant message will be the same for many signatures
and can be cached or pre-calculated.

The message is at most 111 bytes long.

## References

* [SCALE encoding](https://substrate.dev/docs/en/knowledgebase/advanced/codec),
  [github](https://github.com/paritytech/parity-scale-codec)
* [SegWit signatures](https://github.com/bitcoin/bips/blob/master/bip-0143.mediawiki)
* [Taproot signatures](https://github.com/bitcoin/bips/blob/master/bip-0341.mediawiki#signature-validation-rules)
