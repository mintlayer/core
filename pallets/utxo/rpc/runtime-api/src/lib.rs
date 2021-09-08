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
// Author(s): A. Altonen, Anton Sinitsyn
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::inherent::Vec;
// use pallet_utxo_tokens::TokenListData;

sp_api::decl_runtime_apis! {
    pub trait UtxoApi {
        fn send() -> u32;
        fn token_create(name: Vec<u8>, ticker: Vec<u8>, supply: u128) -> u64;
        fn tokens_list() -> Vec<u8>;
    }
}
