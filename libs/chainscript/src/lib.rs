#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
pub mod util;

// TODO: hide implementation of most items exported here
pub mod context;
pub mod error;
pub mod interpreter;
pub mod opcodes;
pub mod script;

pub use error::Result;
