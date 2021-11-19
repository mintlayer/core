#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
    fn lock() -> Weight;
    fn get_npos_voters(v: u32, s: u32, ) -> Weight;
    fn get_npos_targets(v: u32, ) -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn lock() -> u64 {
        (73_865_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(5 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    // Storage: Staking CounterForNominators (r:1 w:0)
    // Storage: Staking CounterForValidators (r:1 w:0)
    // Storage: Staking Validators (r:501 w:0)
    // Storage: Staking Bonded (r:1500 w:0)
    // Storage: Staking Ledger (r:1500 w:0)
    // Storage: Staking SlashingSpans (r:21 w:0)
    // Storage: BagsList ListBags (r:200 w:0)
    // Storage: BagsList ListNodes (r:1000 w:0)
    // Storage: Staking Nominators (r:1000 w:0)
    fn get_npos_voters(v: u32, s: u32, ) -> Weight {
        (0 as Weight)
            // Standard Error: 91_000
            .saturating_add((26_605_000 as Weight).saturating_mul(v as Weight))
            // Standard Error: 3_122_000
            .saturating_add((16_672_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(RocksDbWeight::get().reads(204 as Weight))
            .saturating_add(RocksDbWeight::get().reads((3 as Weight).saturating_mul(v as Weight)))
            .saturating_add(RocksDbWeight::get().reads((1 as Weight).saturating_mul(s as Weight)))
    }
    // Storage: Staking Validators (r:501 w:0)
    fn get_npos_targets(v: u32, ) -> Weight {
        (0 as Weight)
            // Standard Error: 34_000
            .saturating_add((10_558_000 as Weight).saturating_mul(v as Weight))
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().reads((1 as Weight).saturating_mul(v as Weight)))
    }
}