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

use super::*;

use crate::{Pallet as Utxo, Transaction, TransactionInput, TransactionOutput};
use codec::Encode;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::{EventRecord, RawOrigin};
use hex_literal::hex;
use sp_core::{sp_std::vec, sr25519::Public, testing::SR25519, H256, H512};

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    let events = frame_system::Pallet::<T>::events();
    let system_event: <T as frame_system::Config>::Event = generic_event.into();

    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

benchmarks! {
    // only for test
    test_spend {
        // 5Gq2jqhDKtUScUzm9yCJGDDnhYQ8QHuMWiEzzKpjxma9n57R
        let alice_h256 = H256::from([
            210, 191, 75, 132, 77, 254, 253, 103, 114, 168,
            132, 62, 102, 159, 148, 52, 8, 150, 106, 151, 126,
            58, 226, 175, 29, 215, 142, 15, 85, 244, 223, 103
        ]);
        let alice_pub_key = Public::from_h256(alice_h256.clone());

        let genesis_utxo = hex!("931fe49afe365072e71771cd99e13cfb54fa28fad479e23556ff9de6a3dd19a9");
        let genesis_utxo = H256::from(genesis_utxo);

         let mut tx = Transaction {
            inputs: vec![TransactionInput {
                outpoint: genesis_utxo,
                sig_script: H512::zero(),
            }],
            outputs: vec![TransactionOutput::new(50, alice_h256)],
        };

        let alice_sig = frame_support::sp_io::crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();

        tx.inputs[0].sig_script = H512::from(alice_sig);

        let caller: T::AccountId = whitelisted_caller();
    }: spend(RawOrigin::Signed(caller),tx.clone())
    verify {
        assert_last_event::<T>(Event::TransactionSuccess(tx).into());
        assert_eq!(RewardTotal::<T>::get(),50u128);
        assert!(!UtxoStore::<T>::contains_key(genesis_utxo));
    }

    runtime_spend {
        /// ran using mintlayer-node.
        // 0x76584168d10a20084082ed80ec71e2a783abbb8dd6eb9d4893b089228498e9ff
        let alice_h256 = H256::from([
            212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26,
            189, 4, 169, 159, 214,130, 44, 133,88, 133, 76,
            205, 227, 154, 86, 132, 231, 165, 109, 162, 125]
        );
        let alice_pub_key = Public::from_h256(alice_h256.clone());

        let genesis_utxo = H256::from([
             81, 21, 116, 75, 236, 124, 214, 180, 35, 127, 81,
            208, 154, 106, 21, 216, 89, 10, 92, 139, 45, 15,
            227, 227, 206, 59, 82, 197, 34, 147, 181, 76]
        );

        // 0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48
        let bob_h256 = H256::from([
            142, 175, 4, 21, 22, 135, 115, 99, 38, 201, 254, 161,
            126, 37, 252, 82, 135, 97, 54, 147, 201, 18, 144, 156,
            178, 38, 170, 71, 148, 242, 106, 72]
        );
        let bob_pub_key = Public::from_h256(bob_h256.clone());

        // 0x52ffd490c6b266edc96278e7dee680f0bc2454653f2eab823b8d2ec13289d770c7e0a041f32039b7f73b393f3bdd7b09295b56610b59e2165e3d34d83c9ca98f
        let alice_sigscript = H512::from([
            82, 255, 212, 144, 198, 178, 102, 237, 201, 98, 120,
            231, 222, 230, 128, 240, 188, 36, 84, 101, 63, 46,
            171, 130, 59, 141, 46, 193, 50, 137, 215, 112, 199,
            224, 160, 65, 243, 32, 57, 183, 247, 59, 57, 63, 59,
            221, 123, 9, 41, 91, 86, 97, 11, 89, 226, 22, 94, 61,
            52, 216, 60, 156, 169, 143]
        );

        let mut tx = Transaction {
            inputs: vec![ TransactionInput {
                outpoint: genesis_utxo.clone(),
                sig_script: H512::zero()
            }],
            outputs: vec![ TransactionOutput::new(50, bob_h256)]
        };

        tx.inputs[0].sig_script = alice_sigscript;

        let caller: T::AccountId = whitelisted_caller();
    }: spend(RawOrigin::Signed(caller), tx.clone())
    verify {
        assert_last_event::<T>(Event::TransactionSuccess(tx).into());
        assert_eq!(RewardTotal::<T>::get(),50u128);
        assert!(!UtxoStore::<T>::contains_key(genesis_utxo));
    }
}

// only for test
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn spend() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_test_spend::<Test>());
        });
    }
}
