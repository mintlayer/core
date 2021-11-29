//! Autogenerated weights for pallet_utxo
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-06-21, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/mintlayer-core
// benchmark
// --chain
// dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet
// pallet_utxo
// --extrinsic
// spend
// --steps
// 50
// --repeat
// 20
// --output
// .
// --raw

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_core::sp_std::marker::PhantomData;

/// Weight functions for pallet_utxo.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> crate::WeightInfo for WeightInfo<T> {
    fn spend(s: u32) -> Weight {
        (348_270_000 as Weight)
            // Standard Error: 2_000
            //TODO: literally just copying from substrate's
            .saturating_add((1_146_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }

    fn token_create(u: u32) -> Weight {
        // Under construction
        (u as Weight)
            .saturating_add((100 as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }

    fn send_to_address(s: u32) -> Weight {
        (348_270_000 as Weight)
            // Standard Error: 2_000
            //TODO: literally just copying from substrate's
            .saturating_add((1_146_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }

    //TODO this needs a benchmark
    fn unlock_request_for_withdrawal(s: u32) -> Weight {
        (548_270_000 as Weight)
            //TODO: literally just copying from substrate's
            .saturating_add((1_146_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }

    //TODO this needs a benchmark
    fn withdraw_stake(s: u32) -> Weight {
        (548_270_000 as Weight)
            //TODO: literally just copying from substrate's
            .saturating_add((1_146_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
    }
}
