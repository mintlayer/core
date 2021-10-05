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
use chainscript::{opcodes::all as opc, Builder};
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

        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx1));
        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx2));
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
        assert_err!(
            Utxo::spend(Origin::signed(H256::zero()), tx1),
            "output already exists"
        );
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
        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx));
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

        assert_noop!(
            Utxo::spend(Origin::signed(H256::zero()), tx),
            "missing inputs"
        );
    });
}

#[test]
fn attack_with_empty_transactions() {
    new_test_ext().execute_with(|| {
        assert_err!(
            Utxo::spend(Origin::signed(H256::zero()), Transaction::default()), // empty tx
            "no inputs"
        );

        assert_err!(
            Utxo::spend(
                Origin::signed(H256::zero()),
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
            Utxo::spend(Origin::signed(H256::zero()), tx),
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
            Utxo::spend(Origin::signed(H256::zero()), tx),
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
            Utxo::spend(Origin::signed(H256::zero()), tx),
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

        assert_err!(
            Utxo::spend(Origin::signed(H256::zero()), tx),
            "output value overflow"
        );
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
            Utxo::spend(Origin::signed(H256::zero()), tx),
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

        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx.clone()));
        let new_utxo_hash = tx.outpoint(1);

        // then send rest of the tokens to karl (proving that the first tx was successful)
        let mut tx = Transaction {
            inputs: vec![TransactionInput::new_empty(new_utxo_hash)],
            outputs: vec![TransactionOutput::new_pubkey(90, H256::from(karl_pub_key))],
        };

        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = alice_sig.0.to_vec();

        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx));
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
        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx.clone()));

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
        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx.clone()));
    })
}

#[test]
fn test_tokens() {
    let (mut test_ext, alice_pub_key, karl_pub_key) = new_test_ext_and_keys();
    test_ext.execute_with(|| {
        // Let's create a new test token
        let token_id = BlakeTwo256::hash_of(&b"TEST");
        let supply = 1000;
        // Let's make a tx for a new token:
        // * We need at least one input for the fee and one output for a new token.
        // * TokenID for a new token has to be unique.
        let instance = TokenInstance::new_normal(
            token_id,
            b"New token test".to_vec(),
            b"NTT".to_vec(),
            supply,
        );
        let mut first_tx = Transaction {
            inputs: vec![
                // 100 MLT
                tx_input_gen_no_signature(),
            ],
            outputs: vec![
                // 100 a new tokens
                TransactionOutput::new_token(token_id, supply, H256::from(alice_pub_key)),
                // 20 MLT to be paid as a fee, 80 MLT returning
                TransactionOutput::new_pubkey(80, H256::from(alice_pub_key)),
            ],
        };
        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &first_tx.encode()).unwrap();
        first_tx.inputs[0].witness = alice_sig.0.to_vec();
        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), first_tx.clone()));
        // Store a new TokenInstance to the Storage
        <TokenList<Test>>::insert(token_id, Some(instance.clone()));
        dbg!(&<TokenList<Test>>::get(token_id));

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
        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx.clone()));
    });
}

#[test]
fn attack_double_spend_by_tweaking_input() {
    execute_with_alice(|alice_pub_key| {
        // Prepare and send a transaction with a 50-token output
        let drop_script = Builder::new().push_opcode(opc::OP_DROP).into_script();
        let drop_script_hash = BlakeTwo256::hash(drop_script.as_ref());
        let mut tx0 = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![TransactionOutput::new_script_hash(50, drop_script_hash)],
        };
        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx0.encode()).unwrap();
        tx0.inputs[0].witness = alice_sig.0.to_vec();
        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx0.clone()));

        // Create a transaction that spends the same input 10 times by slightly modifying the
        // redeem script.
        let inputs: Vec<_> = (0..10)
            .map(|i| {
                let witness = Builder::new().push_int(1).push_int(i as i64).into_script();
                TransactionInput::new_script(tx0.outpoint(0), drop_script.clone(), witness)
            })
            .collect();
        let tx1 = Transaction {
            inputs: inputs,
            outputs: vec![TransactionOutput::new_pubkey(500, H256::from(alice_pub_key))],
        };
        assert_err!(
            Utxo::spend(Origin::signed(H256::zero()), tx1),
            "each input should be used only once"
        );
    });
}

#[test]
fn test_pubkey_hash() {
    use chainscript::{Builder, Script};
    execute_with_alice(|alice_pub_key| {
        // `pubkey_hash` is hash160(alice_pub_key)
        let pubkey_hash = [
            0x8c, 0x6a, 0xef, 0x68, 0x71, 0x98, 0x61, 0xb3, 0x25, 0x7f, 0x68, 0x44, 0x84, 0xc7,
            0xec, 0x36, 0x27, 0x97, 0xbf, 0x9f,
        ];

        let script = Script::new_p2pkh(&pubkey_hash);
        let mut tx1 = Transaction {
            inputs: vec![tx_input_gen_no_signature()],
            outputs: vec![TransactionOutput::new_pubkey_hash(40, script)],
        };
        let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx1.encode()).unwrap();
        tx1.inputs[0].witness = alice_sig.0.to_vec();

        let mut tx2 = Transaction {
            inputs: vec![TransactionInput::new_script(
                tx1.outpoint(0),
                Builder::new().into_script(),
                Builder::new().into_script(),
            )],
            outputs: vec![TransactionOutput::new_pubkey(20, H256::zero())],
        };

        let sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx2.encode()).unwrap();
        let witness_script = Builder::new()
            .push_slice(&sig.encode())
            .push_slice(&alice_pub_key)
            .into_script();

        tx2.inputs[0].witness = witness_script.into_bytes();

        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx1));
        assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx2));
    })
}

#[test]
fn test_send_to_address() {
    let (mut test_ext, alice_pub_key, _karl_pub_key) = new_test_ext_and_keys();
    test_ext.execute_with(|| {
        // `addr` is bech32-encoded hash160(karl_pub_key)
        let addr = "bc1q7pyaw92rh34mj6flsh7acccd7ayn4wf787ws4m";

        assert_err!(
            Utxo::send_to_address(
                Origin::signed(H256::from(alice_pub_key)),
                0,
                addr.as_bytes().to_vec(),
            ),
            "Value transferred must be larger than zero",
        );

        assert_err!(
            Utxo::send_to_address(
                Origin::signed(H256::from(alice_pub_key)),
                10_000_000,
                addr.as_bytes().to_vec(),
            ),
            "Caller doesn't have enough UTXOs",
        );

        // send UTXO to karl's address
        assert_ok!(Utxo::send_to_address(
            Origin::signed(H256::from(alice_pub_key)),
            40,
            addr.as_bytes().to_vec(),
        ));
    })
}

#[test]
fn nft_test() {
    execute_with_alice(|alice_pub_key| {
        // Let's create a new test nft
        let nft_id = BlakeTwo256::hash_of(&b"TEST");
        let instance = TokenInstance::new_nft(
            nft_id,
            (*b"01010101010101010101010101010101").to_vec(),
            b"http://facebook.com".to_vec(),
            alice_pub_key.to_vec(),
        );

        if let TokenInstance::Nft {
            id,
            data,
            data_url,
            creator_pubkey,
            ..
        } = instance
        {
            let mut tx = Transaction {
                inputs: vec![
                    // 100 MLT
                    tx_input_gen_no_signature(),
                ],
                outputs: vec![TransactionOutput::new_nft(
                    id,
                    data.to_vec(),
                    data_url,
                    H256::from_slice(creator_pubkey.as_slice()),
                )],
            };
            let alice_sig = crypto::sr25519_sign(SR25519, &alice_pub_key, &tx.encode()).unwrap();
            tx.inputs[0].witness = alice_sig.0.to_vec();
            assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx.clone()));
        }

        // it should allow to write and read ?
        // let rsp = await dataToken.readData(firstTokenId);
        // assert.equal(rsp, empty);
        // await dataToken.writeData(firstTokenId, data);
        // rsp = await dataToken.readData(firstTokenId);
        // assert.equal(rsp, data);
    });
}
