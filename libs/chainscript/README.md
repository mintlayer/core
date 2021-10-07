# Chainscript

A rust library providing an interpreter and a set of tools for working with Bitcoinscript-like
bytecode languages.

Code that deals with opcodes and script manipulation is originally [based on the `rust-bitcoin`
project](https://github.com/rust-bitcoin/rust-bitcoin/tree/bd5d875e8ac87). That applies to
the following modules:

* `src/error.rs`
* `src/opcodes.rs`
* `src/script.rs`
* `src/util.rs`

For more detailed docs, see the [module-level comments](src/lib.rs) in the top-level module
or alternatively run `cargo doc --open`.
