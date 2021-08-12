

pub mod currency {
    pub const MILLICENTS: u128 = 1_000_000_000;
    pub const CENTS: u128 = 1_000 * MILLICENTS;
    pub const DOLLARS: u128 = 100 * CENTS;
}

/// Time.
pub mod time {
    use crate::BlockNumber;



    /// This determines the average expected block time that we are targeting.
    /// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
    /// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
    /// up by `pallet_aura` to implement `fn slot_duration()`.
    ///
    /// Change this to adjust the block time.
    pub const MILLISECS_PER_BLOCK: u64 = 60_000;//1 min

    /// Since BABE is probabilistic this is the average expected block time that
        /// we are targeting. Blocks will be produced at a minimum duration defined
        /// by `SLOT_DURATION`, but some slots will not be allocated to any
        /// authority and hence no block will be produced. We expect to have this
        /// block time on average following the defined slot duration and the value
        /// of `c` configured for BABE (where `1 - c` represents the probability of
        /// a slot being empty).
        /// This value is only used indirectly to define the unit constants below
        /// that are expressed in blocks. The rest of the code should use
        /// `SLOT_DURATION` instead (like the Timestamp pallet for calculating the
        /// minimum period).
        ///
        /// If using BABE with secondary slots (default) then all of the slots will
        /// always be assigned, in which case `MILLISECS_PER_BLOCK` and
        /// `SLOT_DURATION` should have the same value.
        ///
        /// <https://research.web3.foundation/en/latest/polkadot/block-production/Babe.html#-6.-practical-results>
    pub const SECS_PER_BLOCK: u64 = MILLISECS_PER_BLOCK / 1000;

    // NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    // 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
    pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

    // NOTE: Currently it is not possible to change the epoch duration after the chain has started.
    //       Attempting to do so will brick block production.
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
    pub const EPOCH_DURATION_IN_SLOTS: u64 = {
        const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

        (EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
    };

    // Time is measured by number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}