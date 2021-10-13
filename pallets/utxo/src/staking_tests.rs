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

use crate::{mock::*, Transaction, TransactionInput, TransactionOutput, UtxoStore, LockedUtxos, Error, StakingCount, StartingPeriod, BlockAuthorRewardAmount, MLTCoinsAvailable};
use codec::Encode;
use frame_support::{
    assert_err, assert_ok,
    sp_io::crypto,
};
use sp_core::{sp_std::vec, testing::SR25519, H256};
use crate::rewards::{update_reward_amount, period_elapsed};

#[test]
fn simple_staking() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, karl_genesis) = keys_and_hashes[1];
        let (alice_pub_key, _) = keys_and_hashes[0];
        let (greg_pub_key, _) = keys_and_hashes[2];

        let utxo = UtxoStore::<Test>::get(karl_genesis).expect("tom's utxo does not exist");

        let tx = Transaction {
            inputs: vec![
                TransactionInput::new_empty(karl_genesis)
            ],
            outputs: vec![
                // KARL (index 1) wants to be a validator. He will use GREG (index 2) as the stash account.
                // minimum value to stake is 10,
                TransactionOutput::new_stake(10, H256::from(greg_pub_key),H256::from(karl_pub_key),vec![2,1]),
                TransactionOutput::new_pubkey(90,H256::from(karl_pub_key))
            ]
        }.sign(&[utxo],0,&karl_pub_key).expect("karl's pub key not found");
        let locked_utxo_hash = tx.outpoint(0);
        let new_utxo_hash = tx.outpoint(1);

        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx));
        assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
        assert!(LockedUtxos::<Test>::contains_key(locked_utxo_hash));
        assert!(StakingCount::<Test>::contains_key(H256::from(alice_pub_key)));
        assert!(StakingCount::<Test>::contains_key(H256::from(karl_pub_key)));
        assert_eq!(StakingCount::<Test>::get(H256::from(karl_pub_key)), Some((1,10)));
    })
}

#[test]
fn less_than_minimum_stake() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, karl_genesis) = keys_and_hashes[1];
        let (greg_pub_key, _) = keys_and_hashes[2];
        let mut tx = Transaction {
            inputs: vec![
                TransactionInput::new_empty(karl_genesis)
            ],
            outputs: vec![
                // KARL (index 1) wants to be a validator. He will use GREG (index 2) as the stash account.
                // minimum value to stake is 10, but KARL only staked 5.
                TransactionOutput::new_stake(5, H256::from(greg_pub_key),H256::from(karl_pub_key),vec![2,1]),
                TransactionOutput::new_pubkey(90,H256::from(karl_pub_key))
            ]
        };
        let karl_sig = crypto::sr25519_sign(SR25519,&karl_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = karl_sig.0.to_vec();


        assert_err!(Utxo::spend(Origin::signed(H256::zero()), tx), "output value must be equal or more than the set minimum stake");

    })
}

#[test]
fn staker_staking_again() {

    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (alice_pub_key, alice_genesis) = keys_and_hashes[0];
        let (greg_pub_key, _) = keys_and_hashes[2];
        let utxo = UtxoStore::<Test>::get(alice_genesis).expect("alice's utxo does not exist");
        let tx = Transaction {
            inputs: vec![
                TransactionInput::new_empty(alice_genesis)
            ],
            outputs: vec![
                // ALICE (index 0) wants to stake again. He will use GREG (index 2) as the stash account.
                TransactionOutput::new_stake(10, H256::from(greg_pub_key),H256::from(alice_pub_key),vec![2,0]),
                TransactionOutput::new_pubkey(90,H256::from(alice_pub_key))
            ]
        }.sign(&[utxo],0, &alice_pub_key).expect(" alice's pub key not found");

        assert_err!(Utxo::spend(Origin::signed(H256::zero()), tx), Error::<Test>::StakingAlreadyExists);
    })
}

#[test]
fn stash_account_is_staking() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (tom_pub_key, tom_genesis) = keys_and_hashes[3];
        let(greg_pub_key, _) = keys_and_hashes[2];

        let utxo = UtxoStore::<Test>::get(tom_genesis).expect("tom's utxo does not exist");
        let tx = Transaction {
            inputs: vec![
                TransactionInput::new_empty(tom_genesis)
            ],
            outputs: vec![
                // TOM (index 3) wants to stake. But he's a stash account already!
                TransactionOutput::new_stake(10, H256::from(greg_pub_key),H256::from(tom_pub_key),vec![2,3]),
                TransactionOutput::new_pubkey(90,H256::from(tom_pub_key))
            ]
        }.sign(&[utxo.clone()],0,&tom_pub_key).expect("tom's public key not found");

        assert_err!(Utxo::spend(Origin::signed(H256::zero()), tx), "CANNOT STAKE. CONTROLLER ACCOUNT IS ACTUALLY A STASH ACCOUNT");
    })
}

#[test]
fn simple_staking_extra() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (alice_pub_key, alice_genesis) = keys_and_hashes[0];
        let utxo = UtxoStore::<Test>::get(alice_genesis).expect("alice's utxo does not exist");
        let tx = Transaction {
            inputs: vec![
                TransactionInput::new_empty(alice_genesis)
            ],
            outputs: vec![
                // ALICE (index 0) wants to add extra stake.
                TransactionOutput::new_stake_extra(20, H256::from(alice_pub_key)),
                TransactionOutput::new_pubkey(70,H256::from(alice_pub_key))
            ]
        }.sign(&[utxo],0,&alice_pub_key).expect(" alice's pub key not found");

        let locked_utxo_hash = tx.outpoint(0);
        let new_utxo_hash = tx.outpoint(1);


        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx));
        assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
        assert!(LockedUtxos::<Test>::contains_key(locked_utxo_hash));
        assert_eq!(StakingCount::<Test>::get(H256::from(alice_pub_key)), Some((2,30)));
    })
}

#[test]
fn non_validator_staking_extra() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (greg_pub_key, greg_genesis) = keys_and_hashes[2];

        let utxo = UtxoStore::<Test>::get(greg_genesis).expect("tom's utxo does not exist");

        let tx = Transaction {
            inputs: vec![
                TransactionInput::new_empty(greg_genesis)
            ],
            outputs: vec![
                // GREG (index 2) wants to stake extra funds. But he's not a validator...
                TransactionOutput::new_stake_extra(20, H256::from(greg_pub_key)),
                TransactionOutput::new_pubkey(100,H256::from(greg_pub_key))
            ]
        }.sign(&[utxo],0,&greg_pub_key).expect("greg's pub key not found");

        assert_err!(Utxo::spend(Origin::signed(H256::zero()), tx), Error::<Test>::NoStakingRecordFound);
    })
}

#[test]
fn pausing_and_withdrawing() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {

        let mut alice_locked_utxo:Vec<H256>= LockedUtxos::<Test>::iter().map(|(key,value)| {
            key
        }).collect();
        let alice_locked_utxo = alice_locked_utxo.pop().unwrap();

        // ALICE (index 0) wants to stop validating.
        let (alice_pub_key, alice_genesis) = keys_and_hashes[0];

        assert_ok!(Utxo::unlock_stake(Origin::signed(H256::zero()),H256::from(alice_pub_key)));

        // increase the block number 6 times, as if new blocks has been created.
        for i in 1 .. 6{
            next_block();
        }
        assert_ok!(Utxo::withdraw_stake(
            Origin::signed(H256::zero()),
            H256::from(alice_pub_key),
            vec![alice_locked_utxo]
        ));

        assert!(!LockedUtxos::<Test>::contains_key(alice_locked_utxo));
        assert_eq!(MLTCoinsAvailable::<Test>::get(),1_001);
    })
}

#[test]
fn non_validator_pausing(){
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, _) = keys_and_hashes[1];
        assert_err!(
            Utxo::unlock_stake(Origin::signed(H256::zero()),H256::from(karl_pub_key)),
            "CANNOT PAUSE. CONTROLLER ACCOUNT DOES NOT EXIST"
        );
    })
}

#[test]
fn non_validator_withdrawing() {

    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {
        let (karl_pub_key, karl_genesis) = keys_and_hashes[1];

        let mut alice_locked_utxo:Vec<H256>= LockedUtxos::<Test>::iter().map(|(key,value)| {
            key
        }).collect();
        let alice_locked_utxo = alice_locked_utxo.pop().unwrap();

        assert_err!(Utxo::withdraw_stake(
            Origin::signed(H256::zero()),
            H256::from(karl_pub_key),
            vec![alice_locked_utxo]
        ), "NoStakingRecordFound");
    })
}

#[test]
fn withdrawing_before_expected_period() {
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {

        let mut alice_locked_utxo:Vec<H256>= LockedUtxos::<Test>::iter().map(|(key,value)| {
            key
        }).collect();
        let alice_locked_utxo = alice_locked_utxo.pop().unwrap();

        // ALICE (index 0) wants to stop validating.
        let (alice_pub_key, alice_genesis) = keys_and_hashes[0];

        assert_ok!(Utxo::unlock_stake(Origin::signed(H256::zero()),H256::from(alice_pub_key)));

        // ALICE is not waiting for the withdrawal period.
        assert_err!(Utxo::withdraw_stake(
            Origin::signed(H256::zero()),
            H256::from(alice_pub_key),
            vec![alice_locked_utxo]
        ), Error::<Test>::InvalidOperation);
    })
}

#[test]
fn withdrawing_unknown_locked_utxo(){
    let (mut test_ext, keys_and_hashes) = multiple_keys_test_ext();
    test_ext.execute_with(|| {

        // ALICE (index 0) wants to stop validating.
        let (alice_pub_key, alice_genesis) = keys_and_hashes[0];

        assert_ok!(Utxo::unlock_stake(Origin::signed(H256::zero()),H256::from(alice_pub_key)));

        // ALICE withdrawing something.
        assert_err!(Utxo::withdraw_stake(
            Origin::signed(H256::zero()),
            H256::from(alice_pub_key),
            vec![H256::random()]
        ), "OutpointDoesNotExist");
    })
}

#[test]
fn reward_reduced() {
    let (mut test_ext, _, _ ) =  alice_test_ext_and_keys();
    test_ext.execute_with(|| {
        assert_eq!(StartingPeriod::<Test>::get(),0);
        assert_eq!(BlockAuthorRewardAmount::<Test>::get(),100);
        assert_eq!(MLTCoinsAvailable::<Test>::get(),1_000);
        // RewardReduction Period is 5; so at block 6, reward should be reduced.
        let time_now = 6;
        period_elapsed::<Test>(time_now);
        assert_eq!(StartingPeriod::<Test>::get(),time_now);

        let reward_amount =  update_reward_amount::<Test>(1_000);
        assert_eq!(BlockAuthorRewardAmount::<Test>::get(), reward_amount);
    })
}