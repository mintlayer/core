// Copyright (c) 2021 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://spdx.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Author(s): A. Altonen
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    dispatch::Vec,
    pallet_prelude::{DispatchError, DispatchResultWithPostInfo},
};
use sp_core::{H256, H512};

pub trait UtxoApi {
    type AccountId;

    fn spend(
        caller: &Self::AccountId,
        value: u128,
        address: H256,
        utxo: H256,
        sig: H512,
    ) -> DispatchResultWithPostInfo;

    fn unlock_request_for_withdrawal(
        caller: &Self::AccountId,
        stash_account: Self::AccountId,
    ) -> DispatchResultWithPostInfo;

    fn withdraw_stake(
        caller: &Self::AccountId,
        stash_account: Self::AccountId,
        outpoints: Vec<H256>,
    ) -> DispatchResultWithPostInfo;

    fn send_conscrit_p2pk(
        caller: &Self::AccountId,
        destination: &Self::AccountId,
        value: u128,
        outpoints: &Vec<H256>,
    ) -> Result<(), DispatchError>;

    fn send_conscrit_c2c(
        caller: &Self::AccountId,
        destination: &Self::AccountId,
        value: u128,
        data: &Vec<u8>,
        outpoints: &Vec<H256>,
    ) -> Result<(), DispatchError>;
}
