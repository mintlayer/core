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
    mock::*, Destination, RewardTotal, TokenList, Transaction, TransactionInput, TransactionOutput,
    UtxoStore, Value,
};
use codec::Encode;
use frame_support::{
    assert_err, assert_noop, assert_ok,
    sp_io::crypto,
    sp_runtime::traits::{BlakeTwo256, Hash},
};
use pallet_utxo_tokens::TokenInstance;
use sp_core::{sp_std::vec, sr25519::Public, testing::SR25519, H256, H512};

fn tx_input_gen_no_signature() -> TransactionInput {
    TransactionInput::new_empty(H256::from(genesis_utxo()))
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
fn pubkey_commitment_hash() {
    let dest = Destination::<u64>::Pubkey(H256::zero());
    assert_eq!(dest.lock_commitment(), &BlakeTwo256::hash(&[]));
}

#[test]
fn test_script_preimage() {
    use chainscript::{opcodes::all as opc, Builder};
    execute_with_alice(|alice_pub_key| {
        // Create a transaction that can be redeemed by revealing a preimage of a hash.
        let password: &[u8] = "Hello!".as_bytes();
        let password_hash = sp_core::hashing::sha2_256(&password);
        let script = Builder::new()
            .push_opcode(opc::OP_SHA256)
            .push_slice(&password_hash)
            .push_opcode(opc::OP_EQUAL)
            .into_script();
        let script_hash: H256 = BlakeTwo256::hash(script.as_ref());
        let witness_script = Builder::new().push_slice(password).into_script();

        let mut tx1 = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![TransactionOutput::new_script_hash(50, script_hash)],
        };
        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx1.encode()).unwrap();
        tx1.inputs[0].witness = alice_sig.0.to_vec();

        let tx2 = Transaction {
            inputs: vec![TransactionInput::new_script(tx1.outpoint(0), script, witness_script)],
            outputs: vec![TransactionOutput::new_script_hash(20, H256::zero())],
        };

        assert_ok!(Utxo::spend(Origin::signed(0), tx1));
        assert_ok!(Utxo::spend(Origin::signed(0), tx2));
    })
}

#[test]
fn test_unchecked_2nd_output() {
    execute_with_alice(|alice_pub_key| {
        // Create and sign a transaction
        let mut tx1 = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![
                TransactionOutput::new_create_pp(0, vec![], vec![]),
                TransactionOutput::new_pubkey(50, H256::from(alice_pub_key)),
            ],
        };
        let alice_sig1 = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx1.encode()).unwrap();
        tx1.inputs[0].witness = alice_sig1.0.to_vec();

        // Calculate output 1 hash.
        let utxo1_hash = tx1.outpoint(1);
        // Now artificially insert utxo1 (that pays to a pubkey) to the output set.
        UtxoStore::<Test>::insert(utxo1_hash, Some(&tx1.outputs[1]));
        // When adding a transaction, the output should be reported as already present.
        assert_err!(Utxo::spend(Origin::signed(0), tx1), "output already exists");
    })
}

#[test]
fn test_simple_tx() {
    execute_with_alice(|alice_pub_key| {
        // Alice wants to send herself a new utxo of value 50.
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![TransactionOutput::new_pubkey(50, H256::from(alice_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = alice_sig.0.to_vec();
        let new_utxo_hash = tx.outpoint(0);

        let init_utxo = genesis_utxo();
        assert!(UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
        assert_ok!(Utxo::spend(Origin::signed(0), tx));
        assert!(!UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
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
            inputs: vec![TransactionInput::new_empty(H256::zero())],
            outputs: vec![TransactionOutput::new_pubkey(50, H256::from(karl_pub_key))],
        };

        let karl_sig = crypto::sr25519_sign(SR25519, &karl_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = karl_sig.0.to_vec();

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
            outputs: vec![TransactionOutput::new_pubkey(100, H256::from(alice_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();

        tx.inputs[0].witness = alice_sig.0.to_vec();
        tx.inputs[1].witness = alice_sig.0.to_vec();

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
            inputs: vec![TransactionInput::new_with_signature(
                H256::from(genesis_utxo()),
                // Just a random signature!
                H512::random(),
            )],
            outputs: vec![TransactionOutput::new_pubkey(100, H256::from(alice_pub_key))],
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
            outputs: vec![TransactionOutput::new_pubkey(0, H256::from(alice_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = alice_sig.0.to_vec();

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
                TransactionOutput::new_pubkey(Value::MAX, H256::from(alice_pub_key)),
                // Attempts to do overflow total output value
                TransactionOutput::new_pubkey(10, H256::from(alice_pub_key)),
            ],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = alice_sig.0.to_vec();

        assert_err!(Utxo::spend(Origin::signed(0), tx), "output value overflow");
    });
}

#[test]
fn attack_by_overspending() {
    execute_with_alice(|alice_pub_key| {
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![
                TransactionOutput::new_pubkey(100, H256::from(alice_pub_key)),
                // Creates 2 new utxo out of thin air
                TransactionOutput::new_pubkey(2, H256::from(alice_pub_key)),
            ],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = alice_sig.0.to_vec();

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
                TransactionOutput::new_pubkey(10, H256::from(karl_pub_key)),
                TransactionOutput::new_pubkey(90, H256::from(alice_pub_key)),
            ],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = alice_sig.0.to_vec();

        assert_ok!(Utxo::spend(Origin::signed(0), tx.clone()));
        let new_utxo_hash = tx.outpoint(1);

        // then send rest of the tokens to karl (proving that the first tx was successful)
        let mut tx = Transaction {
            inputs: vec![TransactionInput::new_empty(new_utxo_hash)],
            outputs: vec![TransactionOutput::new_pubkey(90, H256::from(karl_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = alice_sig.0.to_vec();

        assert_ok!(Utxo::spend(Origin::signed(0), tx));
    });
}

// alice sends 90 tokens to herself and donates 10 tokens for the block authors
#[test]
fn test_reward() {
    execute_with_alice(|alice_pub_key| {
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![TransactionOutput::new_pubkey(90, H256::from(alice_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = alice_sig.0.to_vec();
        assert_ok!(Utxo::spend(Origin::signed(0), tx.clone()));

        // if the previous spend succeeded, there should be one utxo
        // that has a value of 90 and a reward that has a value of 10
        let utxos = UtxoStore::<Test>::iter_values().next().unwrap().unwrap();
        let reward = RewardTotal::<Test>::get();

        assert_eq!(utxos.value, 90);
        assert_eq!(reward, 10);
    })
}

#[test]
fn test_script() {
    execute_with_alice(|alice_pub_key| {
        let mut tx = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![TransactionOutput::new_pubkey(90, H256::from(alice_pub_key))],
        };

        tx.outputs[0].destination = Destination::Pubkey(H256::zero());

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = alice_sig.0.to_vec();
        assert_ok!(Utxo::spend(Origin::signed(0), tx.clone()));
    })
}

#[test]
fn test_tokens() {
    use crate::TokensHigherID;

    let (mut test_ext, alice_pub_key, karl_pub_key) = new_test_ext_and_keys();
    test_ext.execute_with(|| {
        // Let's create a new test token
        let token_id = <TokensHigherID<Test>>::get()
            .checked_add(1)
            .ok_or("All tokens IDs has taken")
            .unwrap();
        // Let's make a tx for a new token:
        // * We need at least one input for the fee and one output for a new token.
        // * TokenID for a new token has to be unique.
        let instance =
            TokenInstance::new(token_id, b"New token test".to_vec(), b"NTT".to_vec(), 1000);
        let mut first_tx = Transaction {
            inputs: vec![
                // 100 MLT
                tx_input_gen_no_signature(),
            ],
            outputs: vec![
                // 100 a new tokens
                TransactionOutput::new_token(token_id, instance.supply, H256::from(alice_pub_key)),
                // 20 MLT to be paid as a fee, 80 MLT returning
                TransactionOutput::new_pubkey(80, H256::from(alice_pub_key)),
            ],
        };
        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &first_tx.encode()).unwrap();
        first_tx.inputs[0].witness = alice_sig.0.to_vec();
        assert_ok!(Utxo::spend(Origin::signed(0), first_tx.clone()));
        // Store a new TokenInstance to the Storage
        <TokenList<Test>>::mutate(|x| {
            if x.iter().find(|&x| x.id == token_id).is_none() {
                x.push(instance.clone())
            } else {
                panic!("the token has already existed with the same id")
            }
        });
        dbg!(&<TokenList<Test>>::get());

        // alice sends 1000 tokens to karl and the rest back to herself 10 tokens
        let utxo_hash_mlt = BlakeTwo256::hash_of(&(&first_tx, 0 as u64));
        let utxo_hash_token = BlakeTwo256::hash_of(&(&first_tx, 1 as u64));

        let mut tx = Transaction {
            inputs: vec![
                TransactionInput::new_empty(utxo_hash_mlt),
                TransactionInput::new_empty(utxo_hash_token),
            ],
            outputs: vec![TransactionOutput::new_token(token_id, 10, H256::from(karl_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        let sig_script = H512::from(alice_sig.clone());
        for input in tx.inputs.iter_mut() {
            input.witness = sig_script.0.to_vec();
        }
        assert_ok!(Utxo::spend(Origin::signed(0), tx.clone()));
    });
}
