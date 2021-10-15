// Copyright (c) 2021 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://spdx.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Author(s): C. Yap

use crate::{Config, Pallet, Event, BlockAuthorRewardAmount, BlockAuthor, MLTCoinsAvailable, TransactionOutput, UtxoStore, StartingPeriod};

use frame_support::traits::Get;
use sp_runtime::traits::{SaturatedConversion, Saturating, BlakeTwo256, Hash, Zero};


/// Returns `true` if reward reduction period has passed; and if a new period has to be created.
pub(crate) fn period_elapsed<T:Config>(time_now: T::BlockNumber) -> bool {
    let starting_period =  <StartingPeriod<T>>::get();
    let time_elapsed = time_now.saturating_sub(starting_period);
    let has_elapse = time_elapsed > T::RewardReductionPeriod::get();

    if has_elapse {
        log::debug!("period has elapsed. Updating the start of period to now");
        <StartingPeriod<T>>::put(time_now);
    }
    has_elapse
}

/// Returns the newly reduced reward amount for a Block Author.
/// How much a reward is reduced, is based on the config's`RewardReductionFraction`.
pub(super) fn update_reward_amount<T:Config>(coins_available:u128) -> u128 {
    let reduction_fraction = T::RewardReductionFraction::get();

    let reward_amount = <BlockAuthorRewardAmount<T>>::take();
    log::info!("current reward amount: {}", reward_amount);
    let reward_amount_deducted = reduction_fraction.mul_ceil(reward_amount);
    if let Some(reward_amount) = reward_amount.checked_sub(reward_amount_deducted) {

        // as long as there is still coins available, set the reward to 1.
        return if reward_amount.is_zero() {
            <BlockAuthorRewardAmount<T>>::put(1);
            1
        }
        else if reward_amount <= coins_available {
            <BlockAuthorRewardAmount<T>>::put(reward_amount);
            reward_amount
        } else {
            <BlockAuthorRewardAmount<T>>::put(coins_available);
            coins_available
        };
    }
    0
}

/// Rewards the block author with a utxo of value based on the `BlockAuthorRewardAmount`.
pub(super) fn reward_block_author<T:Config>(block_number: T::BlockNumber) {
    let coins_available = <MLTCoinsAvailable<T>>::take();

    // give rewards only if there are coins available.
    if coins_available > 0 {
        // give rewards only if a block author is found
        if let Some(block_author) = <BlockAuthor<T>>::take() {
            log::debug!("reward_block_author:: : {:?}", block_author);

            // check if a period has passed.
            // if it has, update the reward amount based on the reduction rate.
            // see RewardReductionFraction
            let reward_amount = if period_elapsed::<T>(block_number) {
                update_reward_amount::<T>(coins_available)
            } else {
                <BlockAuthorRewardAmount<T>>::get()
            };

            // just double check to avoid creating a utxo of value 0
            if !reward_amount.is_zero() {
                let utxo = TransactionOutput::new_pubkey(reward_amount, block_author.clone());

                let hash = {
                    let b_num = block_number.saturated_into::<u64>();
                    BlakeTwo256::hash_of(&(&utxo, b_num, "author_reward"))
                };

                if !<UtxoStore<T>>::contains_key(hash) {
                    <UtxoStore<T>>::insert(hash, Some(utxo.clone()));

                    // deduct the coins transferred to the block author
                    <MLTCoinsAvailable<T>>::put(coins_available - reward_amount);

                    <Pallet<T>>::deposit_event(Event::<T>::BlockAuthorRewarded { value: utxo.value, destination: block_author});
                }
            }
        } else {
            log::warn!("no block author found for block number {:?}", block_number);
        }

        <MLTCoinsAvailable<T>>::put(coins_available);
    }

}