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

use crate::{Config, BlockAuthor};
use frame_support::{traits::FindAuthor,ConsensusEngineId};
use frame_support::traits::ValidatorSet;

/// Get the `H256` version of the BlockAuthor.
/// The BlockAuthor is known by its index position only.
/// To return the actual accountId, the Validators set is required.
pub struct FindAccountFromAuthorIndex<T,Inner,Validators>(core::marker::PhantomData<(T,Inner,Validators)>);


impl<T: Config, Inner: FindAuthor<u32>, Validators:ValidatorSet<T::AccountId>>
FindAuthor<Validators::ValidatorId> for FindAccountFromAuthorIndex<T,Inner, Validators>
{
    fn find_author<'a, I>(digests: I) -> Option<Validators::ValidatorId>
        where
            I: 'a + IntoIterator<Item=(ConsensusEngineId, &'a [u8])> {
        let i = Inner::find_author(digests)?;
        if let Some(authority) = T::authorities().get(i as usize) {
            <BlockAuthor::<T>>::put(Some(*authority));
        }

        let validators = Validators::validators();
        validators.get(i as usize).map(|k| k.clone())
    }
}