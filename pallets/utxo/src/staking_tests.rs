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
    mock::*, Destination, Error, LockedUtxos, StakingCount, Transaction, TransactionInput,
    TransactionOutput, UtxoStore,
};
use codec::Encode;
use frame_support::{assert_err, assert_ok, sp_io::crypto};
use sp_core::{sp_std::vec, testing::SR25519, H256};

// JUST FOR SEEKING BUG IN FUNCTIONAL TEST
// todo: Remove this
#[test]
fn staking_first_time() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, karl_genesis) = keys_and_hashes[1];
        let (alice_pub_key, _) = keys_and_hashes[0];
        let (greg_pub_key, _) = keys_and_hashes[2];

        let utxo = UtxoStore::<Test>::get(karl_genesis).expect("tom's utxo does not exist");
        let tx1 = Transaction {
            inputs: vec![TransactionInput::new_empty(karl_genesis)],
            outputs: vec![TransactionOutput::new_pubkey(100, H256::from(alice_pub_key))],
            time_lock: Default::default(),
        }
        .sign(&[utxo], 0, &karl_pub_key)
        .expect("karl's pub key not found");
        let utxo = &tx1.outputs[0];
        assert_ok!(Utxo::spend(Origin::none(), tx1.clone()));

        let tx2 = Transaction {
            inputs: vec![TransactionInput::new_empty(tx1.outpoint(0))],
            outputs: vec![
                // KARL (index 1) wants to be a validator. He will use GREG (index 2) as the controller account.
                // minimum value to stake is 10,
                TransactionOutput::new_lock_for_staking(
                    90, // 40000 * MLT_UNIT,
                    H256::from(greg_pub_key),
                    H256::from(greg_pub_key),
                    vec![2, 1],
                ),
                TransactionOutput::new_pubkey(
                    10, /*9999 * MLT_UNIT*/
                    H256::from(karl_pub_key),
                ),
            ],
            time_lock: Default::default(),
        }
        .sign(&[utxo.clone()], 0, &alice_pub_key)
        .expect("Alice's pub key not found");
        let new_utxo_hash = tx2.outpoint(1);

        assert_ok!(Utxo::spend(Origin::none(), tx2));
        assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
        assert!(StakingCount::<Test>::contains_key(H256::from(greg_pub_key)));
        assert!(StakingCount::<Test>::contains_key(H256::from(
            alice_pub_key
        )));
        assert_eq!(
            StakingCount::<Test>::get(H256::from(greg_pub_key)),
            Some((1, 90))
        );
    })
}

#[test]
fn simple_staking() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, karl_genesis) = keys_and_hashes[1];
        let (alice_pub_key, _) = keys_and_hashes[0];
        let (greg_pub_key, _) = keys_and_hashes[2];

        let utxo = UtxoStore::<Test>::get(karl_genesis).expect("tom's utxo does not exist");

        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(karl_genesis)],
            outputs: vec![
                // KARL (index 1) wants to be a validator. He will use GREG (index 2) as the controller account.
                // minimum value to stake is 10,
                TransactionOutput::new_lock_for_staking(
                    10,
                    H256::from(karl_pub_key),
                    H256::from(greg_pub_key),
                    vec![2, 1],
                ),
                TransactionOutput::new_pubkey(90, H256::from(karl_pub_key)),
            ],
            time_lock: Default::default(),
        }
        .sign(&[utxo], 0, &karl_pub_key)
        .expect("karl's pub key not found");
        let locked_utxo_hash = tx.outpoint(0);
        let new_utxo_hash = tx.outpoint(1);

        assert_ok!(Utxo::spend(Origin::none(), tx));
        assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
        assert!(LockedUtxos::<Test>::contains_key(locked_utxo_hash));
        assert!(StakingCount::<Test>::contains_key(H256::from(
            alice_pub_key
        )));
        assert!(StakingCount::<Test>::contains_key(H256::from(karl_pub_key)));
        assert_eq!(
            StakingCount::<Test>::get(H256::from(karl_pub_key)),
            Some((1, 10))
        );
    })
}

#[test]
fn less_than_minimum_stake() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, karl_genesis) = keys_and_hashes[1];
        let (greg_pub_key, _) = keys_and_hashes[2];
        let mut tx = Transaction {
            inputs: vec![TransactionInput::new_empty(karl_genesis)],
            outputs: vec![
                // KARL (index 1) wants to be a validator. He will use GREG (index 2) as the controller account.
                // minimum value to stake is 10, but KARL only staked 5.
                TransactionOutput::new_lock_for_staking(
                    5,
                    H256::from(karl_pub_key),
                    H256::from(greg_pub_key),
                    vec![2, 1],
                ),
                TransactionOutput::new_pubkey(90, H256::from(karl_pub_key)),
            ],
            time_lock: Default::default(),
        };
        let karl_sig = crypto::sr25519_sign(SR25519, &karl_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = karl_sig.0.to_vec();

        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "output value must be equal or more than the minimum stake"
        );
    })
}

#[test]
fn non_mlt_staking() {
    use crate::tokens::OutputData;

    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, karl_genesis) = keys_and_hashes[1];
        let (greg_pub_key, _) = keys_and_hashes[2];

        let utxo = UtxoStore::<Test>::get(karl_genesis).expect("kar's utxo does not exist");

        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(karl_genesis)],
            outputs: vec![
                // KARL (index 1) wants to be a validator. He will use GREG (index 2) as the controller account.
                // minimum value to stake is 10, but KARL only staked 5.
                TransactionOutput {
                    value: 10,
                    destination: Destination::LockForStaking {
                        stash_account: H256::from(karl_pub_key),
                        controller_account: H256::from(greg_pub_key),
                        session_key: vec![2, 1],
                    },
                    data: Some(OutputData::TokenIssuanceV1 {
                        token_ticker: "Token".as_bytes().to_vec(),
                        amount_to_issue: 5_000_000_000,
                        // Should be not more than 18 numbers
                        number_of_decimals: 12,
                        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
                    }),
                },
                TransactionOutput::new_pubkey(80, H256::from(karl_pub_key)),
            ],
            time_lock: Default::default(),
        }
        .sign(&[utxo], 0, &karl_pub_key)
        .expect("karl's pub key not found");

        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "only MLT tokens are supported for staking"
        );
    })
}

#[test]
fn controller_staking_again() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (tom_pub_key, tom_genesis) = keys_and_hashes[0];
        let (greg_pub_key, _) = keys_and_hashes[2];
        let utxo = UtxoStore::<Test>::get(tom_genesis).expect("alice's utxo does not exist");
        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(tom_genesis)],
            outputs: vec![
                // ALICE (index 0) wants to stake again. He will use GREG (index 2) as the controller account.
                TransactionOutput::new_lock_for_staking(
                    10,
                    H256::from(tom_pub_key),
                    H256::from(greg_pub_key),
                    vec![2, 0],
                ),
                TransactionOutput::new_pubkey(90, H256::from(tom_pub_key)),
            ],
            time_lock: Default::default(),
        }
        .sign(&[utxo], 0, &tom_pub_key)
        .expect(" tom's pub key not found");

        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "StashAccountAlreadyRegistered"
        );
    })
}

#[test]
fn stash_account_is_staking() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (alice_pub_key, alice_genesis) = keys_and_hashes[0];
        let (greg_pub_key, _) = keys_and_hashes[2];

        let utxo = UtxoStore::<Test>::get(alice_genesis).expect("alice's utxo does not exist");
        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(alice_genesis)],
            outputs: vec![
                // ALice (index 3) wants to stake. But he's a stash account already!
                TransactionOutput::new_lock_for_staking(
                    10,
                    H256::from(alice_pub_key),
                    H256::from(greg_pub_key),
                    vec![2, 3],
                ),
                TransactionOutput::new_pubkey(90, H256::from(alice_pub_key)),
            ],
            time_lock: Default::default(),
        }
        .sign(&[utxo.clone()], 0, &alice_pub_key)
        .expect("alice's public key not found");

        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "StashAccountAlreadyRegistered"
        );
    })
}

#[test]
fn simple_staking_extra() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (alice_pub_key, alice_genesis) = keys_and_hashes[0];
        let (tom_pub_key, _) = keys_and_hashes[3];
        let utxo = UtxoStore::<Test>::get(alice_genesis).expect("alice's utxo does not exist");
        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(alice_genesis)],
            outputs: vec![
                // ALICE (index 0) wants to add extra stake.
                TransactionOutput::new_lock_extra_for_staking(
                    20,
                    H256::from(alice_pub_key),
                    H256::from(tom_pub_key),
                ),
                TransactionOutput::new_pubkey(70, H256::from(alice_pub_key)),
            ],
            time_lock: Default::default(),
        }
        .sign(&[utxo], 0, &alice_pub_key)
        .expect(" alice's pub key not found");

        let locked_utxo_hash = tx.outpoint(0);
        let new_utxo_hash = tx.outpoint(1);

        assert_ok!(Utxo::spend(Origin::none(), tx));
        assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
        assert!(LockedUtxos::<Test>::contains_key(locked_utxo_hash));
        assert_eq!(
            StakingCount::<Test>::get(H256::from(alice_pub_key)),
            Some((2, 30))
        );
    })
}

#[test]
fn non_validator_staking_extra() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (greg_pub_key, greg_genesis) = keys_and_hashes[2];
        let (karl_pub_key, _) = keys_and_hashes[1];

        let utxo = UtxoStore::<Test>::get(greg_genesis).expect("tom's utxo does not exist");

        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(greg_genesis)],
            outputs: vec![
                // GREG (index 2) wants to stake extra funds. But he's not a validator...
                TransactionOutput::new_lock_extra_for_staking(
                    20,
                    H256::from(greg_pub_key),
                    H256::from(karl_pub_key),
                ),
                TransactionOutput::new_pubkey(100, H256::from(greg_pub_key)),
            ],
            time_lock: Default::default(),
        }
        .sign(&[utxo], 0, &greg_pub_key)
        .expect("greg's pub key not found");

        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "StashAccountNotFound"
        );
    })
}

#[test]
fn pausing_and_withdrawing() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let mut alice_locked_utxo: Vec<H256> =
            LockedUtxos::<Test>::iter().map(|(key, _)| key).collect();
        let alice_locked_utxo = alice_locked_utxo.pop().unwrap();

        // ALICE (index 0) wants to stop validating.
        let (alice_pub_key, _) = keys_and_hashes[0];

        assert_ok!(Utxo::unlock_request_for_withdrawal(Origin::signed(
            H256::from(alice_pub_key)
        ),));

        // increase the block number 6 times, as if new blocks has been created.
        for _ in 1..6 {
            next_block();
        }
        assert_ok!(Utxo::withdraw_stake(Origin::signed(H256::from(
            alice_pub_key
        ))));

        assert!(!LockedUtxos::<Test>::contains_key(alice_locked_utxo));
    })
}

#[test]
fn non_validator_pausing() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, _) = keys_and_hashes[1];
        assert_err!(
            Utxo::unlock_request_for_withdrawal(Origin::signed(H256::from(karl_pub_key)),),
            Error::<Test>::StashAccountNotFound
        );
    })
}

#[test]
fn non_validator_withdrawing() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, _) = keys_and_hashes[1];

        assert_err!(
            Utxo::withdraw_stake(Origin::signed(H256::from(karl_pub_key))),
            "StashAccountNotFound"
        );
    })
}

#[test]
fn withdrawing_before_expected_period() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        // ALICE (index 0) wants to stop validating.
        let (alice_pub_key, _) = keys_and_hashes[0];

        assert_ok!(Utxo::unlock_request_for_withdrawal(Origin::signed(
            H256::from(alice_pub_key)
        )));

        // ALICE is not waiting for the withdrawal period.
        assert_err!(
            Utxo::withdraw_stake(Origin::signed(H256::from(alice_pub_key))),
            "not yet time to withdraw"
        );
    })
}

//TODO: add more test scenarios
