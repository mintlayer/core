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

use crate::{mock::*, Transaction, TransactionInput, TransactionOutput, UtxoStore, Value, RewardTotal};
use codec::Encode;
use frame_support::{
    assert_err, assert_noop, assert_ok,
    sp_io::crypto,
    sp_runtime::traits::{BlakeTwo256, Hash},
};
use sp_core::{sp_std::vec, sr25519::Public, testing::SR25519, H256, H512};

fn tx_input_gen_no_signature() -> TransactionInput {
    TransactionInput {
        outpoint: H256::from(GENESIS_UTXO),
        sig_script: H512::zero(),
    }
}

fn execute_with_alice<F>(mut execute: F)
where
    F: FnMut(Public),
{
    new_test_ext().execute_with(|| {
        let alice_pub_key = crypto::sr25519_public_keys(SR25519)[0];
        execute(alice_pub_key);
    })
}

#[test]
fn test_simple_tx() {
    execute_with_alice(|alice_pub_key| {
        // Alice wants to send herself a new utxo of value 50.
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![TransactionOutput::new(50, H256::from(alice_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].sig_script = H512::from(alice_sig);
        let new_utxo_hash = BlakeTwo256::hash_of(&(&tx.encode(), 0 as u64));

        assert_ok!(Utxo::spend(Origin::signed(0), tx));
        assert!(!UtxoStore::<Test>::contains_key(H256::from(GENESIS_UTXO)));
        assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
        assert_eq!(50, UtxoStore::<Test>::get(new_utxo_hash).unwrap().value);
    })
}

#[test]
fn attack_with_sending_to_own_account() {
    let (mut test_ext, _alice, karl_pub_key) = new_test_ext_and_keys();
    test_ext.execute_with(|| {
        // Karl wants to send himself a new utxo of value 50 out of thin air.
        let mut tx = Transaction {
            inputs: vec![TransactionInput {
                outpoint: H256::zero(),
                sig_script: H512::zero(),
            }],
            outputs: vec![TransactionOutput::new(50, H256::from(karl_pub_key))],
        };

        let karl_sig = crypto::sr25519_sign(SR25519, &karl_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].sig_script = H512::from(karl_sig);

        assert_noop!(Utxo::spend(Origin::signed(0), tx), "missing inputs");
    });
}

#[test]
fn attack_with_empty_transactions() {
    new_test_ext().execute_with(|| {
        assert_err!(
            Utxo::spend(Origin::signed(0), Transaction::default()), // empty tx
            "no inputs"
        );

        assert_err!(
            Utxo::spend(
                Origin::signed(0),
                Transaction {
                    inputs: vec![TransactionInput::default()], // an empty tx
                    outputs: vec![]
                }
            ),
            "no outputs"
        );
    });
}

#[test]
fn attack_by_double_counting_input() {
    execute_with_alice(|alice_pub_key| {
        let mut tx = Transaction {
            inputs: vec![
                tx_input_gen_no_signature(),
                // a double spend of the same UTXO!
                tx_input_gen_no_signature(),
            ],
            outputs: vec![TransactionOutput::new(100, H256::from(alice_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();

        tx.inputs[0].sig_script = H512::from(alice_sig.clone());
        tx.inputs[1].sig_script = H512::from(alice_sig);

        assert_err!(
            Utxo::spend(Origin::signed(0), tx),
            "each input should be used only once"
        );
    });
}

#[test]
fn attack_with_invalid_signature() {
    execute_with_alice(|alice_pub_key| {
        let tx = Transaction {
            inputs: vec![TransactionInput {
                outpoint: H256::from(GENESIS_UTXO),
                // Just a random signature!
                sig_script: H512::random(),
            }],
            outputs: vec![TransactionOutput::new(100, H256::from(alice_pub_key))],
        };

        assert_err!(
            Utxo::spend(Origin::signed(0), tx),
            "signature must be valid"
        );
    });
}

#[test]
fn attack_by_permanently_sinking_outputs() {
    execute_with_alice(|alice_pub_key| {
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            //A 0 value output burns this output forever!
            outputs: vec![TransactionOutput::new(0, H256::from(alice_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].sig_script = H512::from(alice_sig);

        assert_noop!(
            Utxo::spend(Origin::signed(0), tx),
            "output value must be nonzero"
        );
    });
}

#[test]
fn attack_by_overflowing_value() {
    execute_with_alice(|alice_pub_key| {
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![
                TransactionOutput::new(Value::MAX, H256::from(alice_pub_key)),
                // Attempts to do overflow total output value
                TransactionOutput::new(10, H256::from(alice_pub_key)),
            ],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].sig_script = H512::from(alice_sig);

        assert_err!(Utxo::spend(Origin::signed(0), tx), "output value overflow");
    });
}

#[test]
fn attack_by_overspending() {
    execute_with_alice(|alice_pub_key| {
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![
                TransactionOutput::new(100, H256::from(alice_pub_key)),
                // Creates 2 new utxo out of thin air
                TransactionOutput::new(2, H256::from(alice_pub_key)),
            ],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].sig_script = H512::from(alice_sig);

        assert_noop!(
            Utxo::spend(Origin::signed(0), tx),
            "output value must not exceed input value"
        );
    })
}

// first send 10 tokens to karl and return the rest back to alice
// then send the rest of the tokens to karl
#[test]
fn tx_from_alice_to_karl() {
    let (mut test_ext, alice_pub_key, karl_pub_key) = new_test_ext_and_keys();
    test_ext.execute_with(|| {
        // alice sends 10 tokens to karl and the rest back to herself
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![
                TransactionOutput::new(10, H256::from(karl_pub_key)),
                TransactionOutput::new(90, H256::from(alice_pub_key)),
            ],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].sig_script = H512::from(alice_sig);

        assert_ok!(Utxo::spend(Origin::signed(0), tx.clone()));
        let new_utxo_hash = BlakeTwo256::hash_of(&(&tx.encode(), 1 as u64));

        // then send rest of the tokens to karl (proving that the first tx was successful)
        let mut tx = Transaction {
            inputs: vec![TransactionInput::new(new_utxo_hash, H512::zero())],
            outputs: vec![TransactionOutput::new(90, H256::from(karl_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].sig_script = H512::from(alice_sig);

        assert_ok!(Utxo::spend(Origin::signed(0), tx));
    });
}

// alice sends 90 tokens to herself and donates 10 tokens for the block authors
#[test]
fn test_reward() {
    execute_with_alice(|alice_pub_key| {
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![TransactionOutput::new(90, H256::from(alice_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].sig_script = H512::from(alice_sig);
        assert_ok!(Utxo::spend(Origin::signed(0), tx.clone()));

        // if the previous spend succeeded, there should be one utxo
        // that has a value of 90 and a reward that has a value of 10
        let utxos = UtxoStore::<Test>::iter_values().next().unwrap().unwrap();
        let reward = RewardTotal::<Test>::get();

        assert_eq!(utxos.value, 90);
        assert_eq!(reward, 10);
    })
}
