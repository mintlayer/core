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


/// The inner `FindAuthor` will point to the BlockAuthor's index position, given the digest.
/// That index position points to an index of a list of validators, which this pallet doesn't have.
/// The `Validators` is a list that can be used to retrieve the account Id.
impl<T: Config, Inner: FindAuthor<u32>, Validators:ValidatorSet<T::AccountId>>
FindAuthor<Validators::ValidatorId> for FindAccountFromAuthorIndex<T,Inner, Validators>
{
    fn find_author<'a, I>(digests: I) -> Option<Validators::ValidatorId>
        where
            I: 'a + IntoIterator<Item=(ConsensusEngineId, &'a [u8])> {

        // here the inner `FindAuthor` determines the block author.
        // An example would be Aura's `FindAuthor` implementation.
        let i = Inner::find_author(digests)?;

        // we use a list of authorities to get the H256 equivalent of the author.
        // An example would be Aura's list of authorities.
        if let Some(authority) = T::authorities().get(i as usize) {
            <BlockAuthor::<T>>::put(Some(*authority));
        }

        // As Block authors need to be validators as well, This validator set will determine the
        // accountId using the same index provided by the inner `FindAuthor`.
        // An example of validators set is found in pallet-session.
        let validators = Validators::validators();
        validators.get(i as usize).map(|k| k.clone())
    }
}