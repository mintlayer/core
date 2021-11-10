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

use crate::{
    convert_to_h256, staking::StakingHelper, tokens::Value, BlockAuthor, Config, Event, Pallet,
    RewardTotal, TransactionOutput, UtxoStore,
};

use frame_support::traits::Get;
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, CheckedDiv, Hash, SaturatedConversion, Zero};
use sp_runtime::Percent;
use sp_std::convert::TryInto;

/// handle event when a block author is found.
impl<T> pallet_authorship::EventHandler<T::AccountId, T::BlockNumber> for Pallet<T>
where
    T: Config + pallet_authorship::Config,
{
    fn note_author(author: T::AccountId) {
        // store the block author. Reward during the `fn finalize()` phase.
        <BlockAuthor<T>>::put(author);
    }

    fn note_uncle(_author: T::AccountId, _age: T::BlockNumber) {
        log::info!("TODO: no support for this. Or is there...?");
    }
}

/// checks at what period the given block number belongs to.
/// If it exceeds to the maximum value of u8 datatype,
/// reduction_fraction is already way over 100%.
fn increase_reduction_fraction<T: Config>(block_number: T::BlockNumber) -> Option<u8> {
    let reduction_period: T::BlockNumber = T::RewardReductionPeriod::get();

    // When reduction_period is not set, there's no such thing as "block period".
    // There's no need to compute for the "current" block period.
    // Reward will not decrease at all.
    if reduction_period.is_zero() {
        return Some(0);
    }

    // get at what period the current block number is at.
    match block_number
        .checked_div(&reduction_period)
        .expect("successfully retrieved current block period.")
        .try_into()
    {
        Ok(result) => Some(result),
        Err(_) => {
            // block_period exceeds the maximum threshold.
            None
        }
    }
}

/// Returns the newly reduced reward amount for a Block Author.
/// How much a reward is reduced, will be based on the config's`RewardReductionFraction`.
fn get_block_author_reward<T: Config>(block_number: T::BlockNumber) -> Value {
    let reduction_fraction = T::RewardReductionFraction::get().deconstruct();
    let last_block_rewarded_period = (100u8 / reduction_fraction) - 1;

    match increase_reduction_fraction::<T>(block_number) {
        None => {
            // cannot increase the current reduction fraction anymore; it has exceeded 100%.
        }
        Some(current_block_period) => {
            // if current block period has not reached to a point where reduction is at 100%
            if current_block_period <= last_block_rewarded_period {
                // compute for the updated reduction % , given the current block period.
                let updated_reduction_fraction =
                    Percent::from_percent(reduction_fraction * current_block_period);

                let reward_amount = T::InitialReward::get();
                return reward_amount - updated_reduction_fraction.mul_ceil(reward_amount);
            }
        }
    }

    T::DefaultMinimumReward::get()
}

fn insert_to_utxo_store<T: Config>(
    block_number: T::BlockNumber,
    block_author: T::AccountId,
    reward: Value,
) {
    match convert_to_h256::<T>(&block_author) {
        Err(e) => {
            log::warn!("failed to find author: {:?}", e);
        }
        Ok(author_h256) => {
            let utxo = TransactionOutput::new_pubkey(reward, author_h256);

            //TODO: https://github.com/mintlayer/core/pull/83#discussion_r742773343
            let hash = {
                let b_num = block_number.saturated_into::<u64>();
                BlakeTwo256::hash_of(&(&utxo, b_num, "author_reward"))
            };

            if !<UtxoStore<T>>::contains_key(hash) {
                // give the same reward to the `pallet-balances` account.
                match T::StakingHelper::reward(block_author, reward) {
                    Err(e) => {
                        log::warn!("failed to reward author's balance: {:?}", e.error);
                        log::warn!(
                            "failed to reward author's balance part 2: {:?}",
                            e.post_info
                        );
                    }
                    Ok(_) => {
                        <UtxoStore<T>>::insert(hash, utxo.clone());
                        <Pallet<T>>::deposit_event(Event::<T>::BlockAuthorRewarded(utxo));
                    }
                }
            }
        }
    }
}

/// Rewards the block author with a utxo of value based on the `BlockAuthorRewardAmount`
/// and the transaction fees.
pub(crate) fn reward_block_author<T: Config>(block_number: T::BlockNumber) {
    // As written on the definition of Take:
    // Take a value from storage, removing it afterwards.
    // This is taking a value of the RewardTotal storage, freeing it up.
    let transaction_fees = <RewardTotal<T>>::take();

    if let Some(reward_amount) =
        get_block_author_reward::<T>(block_number).checked_add(transaction_fees)
    {
        // As written on the definition of Take:
        // Take a value from storage, removing it afterwards.
        // This is taking a value of the BlockAuthor storage, freeing it up.
        let block_author = <BlockAuthor<T>>::take().expect("Block author found.");
        insert_to_utxo_store::<T>(block_number, block_author, reward_amount)
    } else {
        //TODO: what's the actual behaviour (or if this happens at all)
        log::warn!("problem adding the block author reward and the fees.");

        <RewardTotal<T>>::put(transaction_fees);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mock::*;

    #[test]
    fn increase_reduction_fraction_test() {
        alice_test_ext().execute_with(|| {
            // with ReductionPeriod = 5:
            // at Block 3, period is at 0, since 3 < 5
            assert_eq!(increase_reduction_fraction::<Test>(3), Some(0));

            // at Block 3, period is at 0, since 4 < 5
            assert_eq!(increase_reduction_fraction::<Test>(4), Some(0));

            // at Block 5, period is at 1, since 5/ReductionPeriod = 1.
            assert_eq!(increase_reduction_fraction::<Test>(5), Some(1));

            // at Block 3, period is at 2
            assert_eq!(increase_reduction_fraction::<Test>(10), Some(2));

            // at Block 20, period is at 4
            assert_eq!(increase_reduction_fraction::<Test>(20), Some(4));

            // at Block 53, period is at 10, since 53/ReductionPeriod = 10 (nevermind the remainder)
            assert_eq!(increase_reduction_fraction::<Test>(53), Some(10));

            // at Block 42000, multiplication factor of the reduction fraction is way off. (nevermind the remainder)
            assert_eq!(increase_reduction_fraction::<Test>(4200), None);
        });
    }

    #[test]
    fn get_block_author_reward_test() {
        alice_test_ext().execute_with(|| {
            // at Block 1, period is at 0, meaning no deduction of the reward.
            assert_eq!(get_block_author_reward::<Test>(1), 100);

            // at Block 2, period is still at 0, meaning no deduction of the reward.
            assert_eq!(get_block_author_reward::<Test>(2), 100);

            // at Block 5, a new period has started,
            // meaning reward is deducted by RewardReductionFraction: 25% of 100
            assert_eq!(get_block_author_reward::<Test>(5), 75);

            // at Block 9, period is still at 1,
            // meaning reward is deducted by RewardReductionFraction: 25% of 100
            assert_eq!(get_block_author_reward::<Test>(9), 75);

            // at Block 10, a new period has started,
            // meaning reward is deducted by RewardReductionFraction: 25% twice.
            assert_eq!(get_block_author_reward::<Test>(10), 50);

            // at Block 12, period is still at 2,
            // meaning reward is deducted by RewardReductionFraction: 25% twice.
            assert_eq!(get_block_author_reward::<Test>(12), 50);

            // at Block 20, a new period has started.
            // With period set to 5, this makes Block 20 at period 4.
            // meaning reward is deducted by RewardReductionFraction: 25% 4 times.. or by 100%.
            // in testnet, when reduced reward approaches 0, the reward will become 1.
            assert_eq!(get_block_author_reward::<Test>(20), 1);

            // exceeds the reduction fraction of 100%, so reward is constantly at 1 based on testnet.
            assert_eq!(get_block_author_reward::<Test>(68), 1);

            // exceeds the reduction fraction of 100%, so reward is constantly at 1 based on testnet.
            assert_eq!(get_block_author_reward::<Test>(5000), 1);
        });
    }
}
