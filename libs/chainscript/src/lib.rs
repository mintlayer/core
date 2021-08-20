#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
mod util;

// TODO: hide implementation of most items exported here
pub mod error;
pub mod opcodes;
pub mod script;

pub use error::Result;
