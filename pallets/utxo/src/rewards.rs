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
    convert_to_h256, BlockAuthor, Config, Event, Pallet, RewardTotal, TransactionOutput, UtxoStore,
    Value,
};

use frame_support::traits::Get;
use sp_runtime::traits::{BlakeTwo256, CheckedDiv, Hash, SaturatedConversion};
use sp_runtime::Percent;
use sp_std::convert::TryInto;
use sp_core::H256;

/// handle event when a block author is found.
impl<T> pallet_authorship::EventHandler<T::AccountId, T::BlockNumber> for Pallet<T>
where
    T: Config + pallet_authorship::Config,
{
    fn note_author(author: T::AccountId) {
        match convert_to_h256::<T>(&author) {
            Ok(author_h256) => {
                // store the block author. Reward during the `fn finalize()` phase.
                <BlockAuthor<T>>::put(author_h256);
            }
            Err(e) => {
                log::warn!("failed to find author: {:?}", e);
            }
        }
    }

    fn note_uncle(_author: T::AccountId, _age: T::BlockNumber) {
        log::info!("TODO: no support for this. Or is there...?");
    }
}


/// checks at what period the given block number belongs to.
fn get_block_period<T:Config>(block_number: T::BlockNumber) -> u8 {
    let reduction_period: T::BlockNumber = T::RewardReductionPeriod::get();

    // get at what period the current block number is at.
    block_number.checked_div(&reduction_period)
        .map_or(u8::MAX, |f|{
            // convert data type from T::BlockNumber to u8
            f.try_into().unwrap_or(u8::MAX)
        })
}


/// Returns the newly reduced reward amount for a Block Author.
/// How much a reward is reduced, will be based on the config's`RewardReductionFraction`.
fn get_block_author_reward<T: Config>(block_number: T::BlockNumber) -> Value {
    let reward_amount = T::InitialReward::get();
    let reduction_period: T::BlockNumber = T::RewardReductionPeriod::get();
    let reduction_fraction = T::RewardReductionFraction::get().deconstruct();

    // no computation needed; current block number is still within the reduction period.
    if block_number < reduction_period {
        return reward_amount;
    }

    // The maximum period before approaching to 100% reduction
    let max_period = (100u8 / reduction_fraction) - 1;

    // current period of the given block number
    let current_period = get_block_period::<T>(block_number);

    // when current_period has reached or exceeded the maximum period of reduction,
    // return the least amount to reward.
    if current_period > max_period {
        // TODO: as a note, this is only testnet specific to reward at least 1
        return 1;
    }

    // compute for the updated reduction % , given the current period.
    let updated_reduction_fraction = Percent::from_percent(reduction_fraction * current_period );

    reward_amount - updated_reduction_fraction.mul_ceil(reward_amount)
}

fn insert_to_utxo_store<T:Config>(block_number:T::BlockNumber, block_author:H256, reward:Value) {
    let utxo = TransactionOutput::new_pubkey(reward, block_author.clone());

    let hash = {
        let b_num = block_number.saturated_into::<u64>();
        BlakeTwo256::hash_of(&(&utxo, b_num, "author_reward"))
    };

    if !<UtxoStore<T>>::contains_key(hash) {
        <UtxoStore<T>>::insert(hash, utxo.clone());

        <Pallet<T>>::deposit_event(Event::<T>::BlockAuthorRewarded(utxo));
    }
}

/// Rewards the block author with a utxo of value based on the `BlockAuthorRewardAmount`
/// and the transaction fees.
pub(crate) fn reward_block_author<T: Config>(block_number: T::BlockNumber) {

    // As written on the definition of Take:
    // Take a value from storage, removing it afterwards.
    // This is taking a value of the RewardTotal storage, freeing it up.
    let transaction_fees = <RewardTotal<T>>::take();

    if let Some(reward_amount) = get_block_author_reward::<T>(block_number).checked_add(transaction_fees) {
        // As written on the definition of Take:
        // Take a value from storage, removing it afterwards.
        // This is taking a value of the BlockAuthor storage, freeing it up.
        match <BlockAuthor<T>>::take() {
            None => {
                log::warn!("no block author found for block number {:?}", block_number);
                // carry over the fees to the next block rewarding.
                <RewardTotal<T>>::put(reward_amount);
            }
            Some(block_author) => {
                insert_to_utxo_store::<T>(block_number,block_author, reward_amount)
            }
        }
    } else {
        //TODO: what's the actual behaviour (or if this happens at all)
        log::warn!("problem adding the block author reward and the fees.");

        <RewardTotal<T>>::put(transaction_fees);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::*;

    #[test]
    fn get_block_period_test() {
        alice_test_ext().execute_with(|| {
            // with ReductionPeriod = 5:
            // at Block 3, period is at 0, since 3 < 5
            assert_eq!(get_block_period::<Test>(3),0);

            // at Block 3, period is at 0, since 4 < 5
            assert_eq!(get_block_period::<Test>(4),0);

            // at Block 5, period is at 1, since 5/ReductionPeriod = 1.
            assert_eq!(get_block_period::<Test>(5),1);

            // at Block 3, period is at 2
            assert_eq!(get_block_period::<Test>(10),2);

            // at Block 20, period is at 4
            assert_eq!(get_block_period::<Test>(20),4);

            // at Block 53, period is at 10, since 53/ReductionPeriod = 10 (nevermind the remainder)
            assert_eq!(get_block_period::<Test>(53),10);
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
        });
    }
}