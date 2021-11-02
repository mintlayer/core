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
};

use frame_support::traits::Get;
use sp_runtime::traits::{
    BlakeTwo256, CheckedDiv, CheckedSub, Hash, One, SaturatedConversion, Zero,
};

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

/// Returns the newly reduced reward amount for a Block Author.
/// How much a reward is reduced, is based on the config's`RewardReductionFraction`.
fn get_reward_amount<T: Config>(block_number: T::BlockNumber) -> u128 {
    let reduction_fraction = T::RewardReductionFraction::get();
    let reduction_period: T::BlockNumber = T::RewardReductionPeriod::get();
    let mut reward_amount = T::InitialReward::get();

    if let Some(mut counter) = block_number.checked_div(&reduction_period) {
        // how many times the initial reward should be slashed.
        while counter > T::BlockNumber::zero() {
            counter = counter.checked_sub(&T::BlockNumber::one()).unwrap_or(T::BlockNumber::zero());

            reward_amount = reward_amount
                .checked_sub(reduction_fraction.mul_ceil(reward_amount))
                .unwrap_or(1); // TODO: this is only testnet specific to reward at least 1

            if reward_amount.is_zero() {
                // TODO: this is only testnet specific to reward at least 1
                return 1;
            }
        }
    }

    reward_amount
}

/// Rewards the block author with a utxo of value based on the `BlockAuthorRewardAmount`
/// and the transaction fees.
pub(crate) fn reward_block_author<T: Config>(block_number: T::BlockNumber) {
    let fees_total = <RewardTotal<T>>::take();
    if let Some(reward_amount) = get_reward_amount::<T>(block_number).checked_add(fees_total) {
        // give rewards only if a block author is found
        if let Some(block_author) = <BlockAuthor<T>>::take() {
            let utxo = TransactionOutput::new_pubkey(reward_amount, block_author.clone());

            let hash = {
                let b_num = block_number.saturated_into::<u64>();
                BlakeTwo256::hash_of(&(&utxo, b_num, "author_reward"))
            };

            if !<UtxoStore<T>>::contains_key(hash) {
                <UtxoStore<T>>::insert(hash, utxo.clone());

                <Pallet<T>>::deposit_event(Event::<T>::BlockAuthorRewarded(utxo));
            }
        } else {
            log::warn!("no block author found for block number {:?}", block_number);
            <RewardTotal<T>>::put(fees_total + reward_amount);
        }
    } else {
        log::warn!("problem adding the block author reward and the fees.");
        <RewardTotal<T>>::put(fees_total);
    }
}
