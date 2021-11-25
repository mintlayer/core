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
    mock::*, tokens::Value, BlockTime, Destination, RawBlockTime, RewardTotal, Transaction,
    TransactionInput, TransactionOutput, UtxoStore,
};
use chainscript::{opcodes::all as opc, Builder};
use codec::Encode;
use frame_support::{
    assert_err, assert_noop, assert_ok,
    sp_io::crypto,
    sp_runtime::traits::{BlakeTwo256, Hash},
};

use crate::script::test::gen_block_time_real;
use crate::tokens::OutputData;
use proptest::prelude::*;
use sp_core::{sp_std::vec, sr25519::Public, testing::SR25519, H256, H512};

fn tx_input_gen_no_signature() -> (TransactionOutput<H256>, TransactionInput) {
    let (utxo, hash) = genesis_utxo();
    (utxo, TransactionInput::new_empty(hash))
}

fn execute_with_alice<F, R>(mut execute: F) -> R
where
    F: FnMut(Public) -> R,
{
    alice_test_ext().execute_with(|| {
        let alice_pub_key = crypto::sr25519_public_keys(SR25519)[0];
        execute(alice_pub_key)
    })
}

impl<AccountId: Encode> Transaction<AccountId> {
    // A convenience method to sign a transaction. Only Schnorr supported for now.
    fn sign_unchecked(
        self,
        utxos: &[TransactionOutput<AccountId>],
        index: usize,
        pk: &Public,
    ) -> Self {
        self.sign(utxos, index, pk).expect("Public key not found")
    }
}

#[test]
fn pubkey_commitment_hash() {
    let dest = Destination::<u64>::Pubkey(Public([0; 32]).into());
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

        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx1 = Transaction {
            inputs: vec![input0],
            outputs: vec![TransactionOutput::new_script_hash(
                ALICE_GENESIS_BALANCE - 50,
                script_hash,
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);

        let tx2 = Transaction {
            inputs: vec![TransactionInput::new_script(tx1.outpoint(0), script, witness_script)],
            outputs: vec![TransactionOutput::new_script_hash(
                ALICE_GENESIS_BALANCE - 120,
                H256::zero(),
            )],
            time_lock: Default::default(),
        };

        assert_ok!(Utxo::spend(Origin::none(), tx1));
        assert_ok!(Utxo::spend(Origin::none(), tx2));
    })
}

#[test]
fn test_unchecked_2nd_output() {
    execute_with_alice(|alice_pub_key| {
        // Create and sign a transaction
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx1 = Transaction {
            inputs: vec![input0],
            outputs: vec![
                TransactionOutput::new_pubkey(
                    ALICE_GENESIS_BALANCE - 30,
                    H256::from(alice_pub_key),
                ),
                TransactionOutput::new_pubkey(
                    ALICE_GENESIS_BALANCE - 50,
                    H256::from(alice_pub_key),
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);

        // Calculate output 1 hash.
        let utxo1_hash = tx1.outpoint(1);
        // Now artificially insert utxo1 (that pays to a pubkey) to the output set.
        UtxoStore::<Test>::insert(utxo1_hash, &tx1.outputs[1]);
        // When adding a transaction, the output should be reported as already present.
        assert_err!(
            Utxo::spend(Origin::none(), tx1),
            "output already exists"
        );
    })
}

#[test]
fn test_simple_tx() {
    execute_with_alice(|alice_pub_key| {
        // Alice wants to send herself a new utxo of value 50.
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![TransactionOutput::new_pubkey(
                ALICE_GENESIS_BALANCE - 50,
                H256::from(alice_pub_key),
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);

        let new_utxo_hash = tx.outpoint(0);

        let (_, init_utxo) = genesis_utxo();
        assert!(UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
        assert_ok!(Utxo::spend(Origin::none(), tx));
        assert!(!UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
        assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
        assert_eq!(
            ALICE_GENESIS_BALANCE - 50,
            UtxoStore::<Test>::get(new_utxo_hash).unwrap().value
        );
    })
}

#[test]
fn attack_with_sending_to_own_account() {
    let (mut test_ext, _alice, karl_pub_key) = alice_test_ext_and_keys();
    test_ext.execute_with(|| {
        // Karl wants to send himself a new utxo of value 50 out of thin air.
        let mut tx = Transaction {
            inputs: vec![TransactionInput::new_empty(H256::zero())],
            outputs: vec![TransactionOutput::new_pubkey(50, H256::from(karl_pub_key))],
            time_lock: Default::default(),
        };

        let karl_sig = crypto::sr25519_sign(SR25519, &karl_pub_key, &tx.encode()).unwrap();
        tx.inputs[0].witness = karl_sig.0.to_vec();

        assert_noop!(
            Utxo::spend(Origin::none(), tx),
            "missing inputs"
        );
    });
}

#[test]
fn attack_with_empty_transactions() {
    alice_test_ext().execute_with(|| {
        // We should use the real input because. Otherwise, appears another error
        let (_, input) = tx_input_gen_no_signature();
        assert_err!(
            Utxo::spend(Origin::none(), Transaction::default()), // empty tx
            "no inputs"
        );

        assert_err!(
            Utxo::spend(
                Origin::none(),
                Transaction {
                    inputs: vec![input], // an empty tx
                    outputs: vec![],
                    time_lock: Default::default()
                }
            ),
            "no outputs"
        );
    });
}

#[test]
fn attack_by_double_counting_input() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();
        let utxos = [utxo0.clone(), utxo0];
        let tx = Transaction {
            // a double spend of the same UTXO!
            inputs: vec![input0.clone(), input0],
            outputs: vec![TransactionOutput::new_pubkey(100, H256::from(alice_pub_key))],
            time_lock: Default::default(),
        }
        .sign_unchecked(&utxos[..], 0, &alice_pub_key)
        .sign_unchecked(&utxos[..], 1, &alice_pub_key);

        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "each input should be used only once"
        );
    });
}

#[test]
fn attack_with_invalid_signature() {
    execute_with_alice(|alice_pub_key| {
        let (_utxo0, input0) = genesis_utxo();
        let tx = Transaction {
            // Just a random signature!
            inputs: vec![TransactionInput::new_with_signature(input0, H512::random())],
            outputs: vec![TransactionOutput::new_pubkey(100, H256::from(alice_pub_key))],
            time_lock: Default::default(),
        };

        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "signature must be valid"
        );
    });
}

#[test]
fn attack_by_permanently_sinking_outputs() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            //A 0 value output burns this output forever!
            outputs: vec![TransactionOutput::new_pubkey(0, H256::from(alice_pub_key))],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);

        assert_noop!(
            Utxo::spend(Origin::none(), tx),
            "output value must be nonzero"
        );
    });
}

#[test]
fn attack_by_overflowing_value() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![
                TransactionOutput::new_pubkey(Value::MAX, H256::from(alice_pub_key)),
                // Attempts to do overflow total output value
                TransactionOutput::new_pubkey(10, H256::from(alice_pub_key)),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);

        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "output value overflow"
        );
    });
}

#[test]
fn attack_by_overspending() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![
                TransactionOutput::new_pubkey(ALICE_GENESIS_BALANCE, H256::from(alice_pub_key)),
                // Creates 2 new utxo out of thin air
                TransactionOutput::new_pubkey(2, H256::from(alice_pub_key)),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);

        assert_noop!(
            Utxo::spend(Origin::none(), tx),
            "output value must not exceed input value"
        );
    })
}

// first send 10 tokens to karl and return the rest back to alice
// then send the rest of the tokens to karl
#[test]
fn tx_from_alice_to_karl() {
    let (mut test_ext, alice_pub_key, karl_pub_key) = alice_test_ext_and_keys();
    test_ext.execute_with(|| {
        // alice sends 10 tokens to karl and the rest back to herself
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![
                TransactionOutput::new_pubkey(10, H256::from(karl_pub_key)),
                TransactionOutput::new_pubkey(
                    ALICE_GENESIS_BALANCE - 90,
                    H256::from(alice_pub_key),
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);

        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let new_utxo_hash = tx.outpoint(1);
        let new_utxo = tx.outputs[1].clone();

        // then send rest of the tokens to karl (proving that the first tx was successful)
        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(new_utxo_hash)],
            outputs: vec![TransactionOutput::new_pubkey(
                ALICE_GENESIS_BALANCE - 90,
                H256::from(karl_pub_key),
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[new_utxo], 0, &alice_pub_key);

        assert_ok!(Utxo::spend(Origin::none(), tx));
    });
}

// alice sends 90 tokens to herself and donates 10 tokens for the block authors
#[test]
fn test_reward() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();

        // Check the default parameters
        let utxos = UtxoStore::<Test>::get(input0.outpoint).unwrap();
        assert_eq!(utxos.value, ALICE_GENESIS_BALANCE);
        let reward = RewardTotal::<Test>::get();
        assert_eq!(reward, 0);

        // Make a new transaction
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![TransactionOutput::new_pubkey(
                ALICE_GENESIS_BALANCE - 90,
                H256::from(alice_pub_key),
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);

        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let utxo_hash = tx.outpoint(0);

        // if the previous spend succeeded, there should be one utxo
        // that has a value of ALICE_GENESIS_BALANCE - 90 and a reward that has a value of 90
        let utxos = UtxoStore::<Test>::get(utxo_hash).unwrap();
        let reward = RewardTotal::<Test>::get();
        assert_eq!(utxos.value, ALICE_GENESIS_BALANCE - 90);
        assert_eq!(reward, 90);
    })
}

#[test]
fn test_reward_overflow() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();

        // Check the default parameters
        let utxos = UtxoStore::<Test>::get(input0.outpoint).unwrap();
        assert_eq!(utxos.value, ALICE_GENESIS_BALANCE);
        let reward = RewardTotal::<Test>::get();
        assert_eq!(reward, 0);

        // Make a new transaction where
        // Input balance:  4_000_000_000_000_000_000_000
        // u64::MAX:          18_446_744_073_709_551_615
        // the difference: 3_981_553_255_926_290_448_385
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![TransactionOutput::new_pubkey(
                3_981_553_255_926_290_448_385,
                H256::from(alice_pub_key),
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);
        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "reward exceed allowed amount"
        );
    })
}

#[test]
fn test_script() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![TransactionOutput::new_pubkey(
                ALICE_GENESIS_BALANCE - 90,
                H256::from(alice_pub_key),
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);

        assert_ok!(Utxo::spend(Origin::none(), tx));
    })
}

#[test]
fn test_time_lock_tx() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![TransactionOutput::new_pubkey(
                ALICE_GENESIS_BALANCE - 90,
                H256::from(alice_pub_key),
            )],
            time_lock: BlockTime::Blocks(10).as_raw().unwrap(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);
        assert_err!(
            Utxo::spend(Origin::none(), tx),
            "Time lock restrictions not satisfied",
        );
    })
}

#[test]
fn test_time_lock_script_fail() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();
        let script = Builder::new().push_int(10).push_opcode(opc::OP_CLTV).into_script();
        let script_hash: H256 = BlakeTwo256::hash(script.as_ref());
        let tx1 = Transaction {
            inputs: vec![input0],
            outputs: vec![TransactionOutput::new_script_hash(
                ALICE_GENESIS_BALANCE - 90,
                script_hash,
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);
        let outpoint = tx1.outpoint(0);
        assert_ok!(Utxo::spend(Origin::none(), tx1));

        // The following should fail because the transaction-level time lock does not conform to
        // the time lock restrictions imposed by the scripting system.
        let tx2 = Transaction {
            inputs: vec![TransactionInput::new_script(outpoint, script, Default::default())],
            outputs: vec![TransactionOutput::new_pubkey(
                ALICE_GENESIS_BALANCE - 150,
                H256::from(alice_pub_key),
            )],
            time_lock: Default::default(),
        };
        assert_err!(
            Utxo::spend(Origin::none(), tx2),
            "script verification failed"
        );
    })
}

#[test]
fn attack_double_spend_by_tweaking_input() {
    execute_with_alice(|alice_pub_key| {
        // Prepare and send a transaction with a 50-token output
        let (utxo0, input0) = tx_input_gen_no_signature();
        let drop_script = Builder::new().push_opcode(opc::OP_DROP).into_script();
        let drop_script_hash = BlakeTwo256::hash(drop_script.as_ref());
        let tx0 = Transaction {
            inputs: vec![input0],
            outputs: vec![TransactionOutput::new_script_hash(
                ALICE_GENESIS_BALANCE - 50,
                drop_script_hash,
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);
        assert_ok!(Utxo::spend(Origin::none(), tx0.clone()));

        // Create a transaction that spends the same input 10 times by slightly modifying the
        // redeem script.
        let inputs: Vec<_> = (0..10)
            .map(|i| {
                let witness = Builder::new().push_int(1).push_int(i as i64).into_script();
                TransactionInput::new_script(tx0.outpoint(0), drop_script.clone(), witness)
            })
            .collect();
        let tx1 = Transaction {
            inputs,
            outputs: vec![TransactionOutput::new_pubkey(
                ALICE_GENESIS_BALANCE - 500,
                H256::from(alice_pub_key),
            )],
            time_lock: Default::default(),
        };
        assert_err!(
            Utxo::spend(Origin::none(), tx1),
            "each input should be used only once"
        );
    });
}

#[test]
fn test_send_to_address() {
    let (mut test_ext, alice_pub_key, _karl_pub_key) = alice_test_ext_and_keys();
    test_ext.execute_with(|| {
        // `addr` is bech32-encoded, SCALE-encoded `Destination::Pubkey(alice_pub_key)`
        let addr = "ml1qrft7juyfhl06emj4zzrue5ljs6q39n2jalr4c40rhtcur647n0kwueyfsn";

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
                ALICE_GENESIS_BALANCE * 10,
                addr.as_bytes().to_vec(),
            ),
            "Caller doesn't have enough UTXOs",
        );

        // send 10 utxo to alice
        assert_ok!(Utxo::send_to_address(
            Origin::signed(H256::from(alice_pub_key)),
            10,
            addr.as_bytes().to_vec(),
        ));

        // try to transfer to scripthash
        let addr = "ml1qvvknne0acfzfd2ewksccgrgl4qlhcwewq4gjm75mtcpg26al66d5l5sz9k";
        assert_ok!(Utxo::send_to_address(
            Origin::signed(H256::from(alice_pub_key)),
            20,
            addr.as_bytes().to_vec(),
        ));

        // invalid length
        let addr = "ml1qrft7juyfhl06emj4zzrue5ljs6q39n2jalr4c40rhtcur647n0kwue1yfsn";
        assert_err!(
            Utxo::send_to_address(
                Origin::signed(H256::from(alice_pub_key)),
                40,
                addr.as_bytes().to_vec(),
            ),
            "Failed to decode address: invalid length",
        );

        // invalid character
        let addr = "ml1qyzqqpqpäääypw";
        assert_err!(
            Utxo::send_to_address(
                Origin::signed(H256::from(alice_pub_key)),
                40,
                addr.as_bytes().to_vec(),
            ),
            "Failed to decode address: invalid character",
        );

        // mixed case
        let addr = "ml1qrft7juyfhl06emj4zzrue5ljs6q39n2JALR4c40rhtcur647n0kWUYEFSN";
        assert_err!(
            Utxo::send_to_address(
                Origin::signed(H256::from(alice_pub_key)),
                40,
                addr.as_bytes().to_vec(),
            ),
            "Failed to decode address: mixed case",
        );

        // invalid checksum
        let addr = "ml1qrft7juyfhl06emj4zzrue5ljs6q39n2jalr4c40rhtcur647n0kwueyf66";
        assert_err!(
            Utxo::send_to_address(
                Origin::signed(H256::from(alice_pub_key)),
                40,
                addr.as_bytes().to_vec(),
            ),
            "Failed to decode address: invalid checksum",
        );

        // invalid HRP
        let addr = "bc1qrft7juyfhl06emj4zzrue5ljs6q39n2jalr4c40rhtcur647n0kwueyfsn";
        assert_err!(
            Utxo::send_to_address(
                Origin::signed(H256::from(alice_pub_key)),
                40,
                addr.as_bytes().to_vec(),
            ),
            "Failed to decode address: invalid HRP",
        );
    })
}

// Proptest config to decrease the number of test runs by the factor of 16 (for expensive tests).
fn proptest_expensive() -> proptest::test_runner::Config {
    let mut config = proptest::test_runner::Config::default();
    config.cases /= 16;
    config
}

proptest! {
    // These tests are fairly expensive, run fewer of them.
    #![proptest_config(proptest_expensive())]

    #[test]
    fn prop_gen_block_time_real_works(bt in gen_block_time_real()) {
        // This generator should not sample block-based time.
        prop_assert!(!bt.is_blocks());
    }

    #[test]
    fn prop_time_lock_realtime_with_script(
        script_lock_time in gen_block_time_real(),
        tx_lock_time in gen_block_time_real(),
        current_time in gen_block_time_real(),
    ) {
        let result = execute_with_alice(|alice| {
            // Convert seconds to milliseconds
            Timestamp::set_timestamp(current_time.as_u64() * 1000);

            let (utxo0, input0) = tx_input_gen_no_signature();
            let script = Builder::new()
                .push_int(script_lock_time.as_u64() as i64)
                .push_opcode(opc::OP_CLTV)
                .into_script();
            let script_hash: H256 = BlakeTwo256::hash(script.as_ref());
            let tx1 = Transaction {
                inputs: vec![input0],
                outputs: vec![TransactionOutput::new_script_hash(ALICE_GENESIS_BALANCE - 90, script_hash)],
                time_lock: Default::default(),
            }
            .sign_unchecked(&[utxo0], 0, &alice);
            let outpoint = tx1.outpoint(0);
            assert!(Utxo::spend(Origin::none(), tx1).is_ok());

            let tx2 = Transaction {
                inputs: vec![TransactionInput::new_script(outpoint, script, Default::default())],
                outputs: vec![TransactionOutput::new_pubkey(ALICE_GENESIS_BALANCE - u32::MAX as Value, H256::from(alice))],
                time_lock: tx_lock_time,
            };
            Utxo::spend(Origin::none(), tx2)
        });

        // The transaction should be accepted if and only if:
        // current time >= transaction lock time >= script lock time
        let model = current_time.as_u64() >= tx_lock_time.as_u64()
            && tx_lock_time.as_u64() >= script_lock_time.as_u64();
        prop_assert_eq!(result.is_ok(), model);
    }

    #[test]
    fn prop_time_lock_realtime_monotonic(
        tx_lock_time in gen_block_time_real(),
        time0 in gen_block_time_real(),
        time1 in gen_block_time_real(),
    ) {
        // Make sure time0 and time1 are in order
        let (time0, time1) = (
            std::cmp::min_by_key(time0, time1, RawBlockTime::as_u64),
            std::cmp::max_by_key(time0, time1, RawBlockTime::as_u64),
        );
        let (res0, res1) = execute_with_alice(|alice| {
            let (utxo0, input0) = tx_input_gen_no_signature();
            let tx = Transaction {
                inputs: vec![input0],
                outputs: vec![TransactionOutput::new_pubkey(ALICE_GENESIS_BALANCE - 50, H256::from(alice))],
                time_lock: tx_lock_time,
            }
            .sign_unchecked(&[utxo0], 0, &alice);

            Timestamp::set_timestamp(time0.as_u64() * 1000);
            let res0 = crate::pallet::validate_transaction::<Test>(&tx);

            Timestamp::set_timestamp(time1.as_u64() * 1000);
            let res1 = crate::pallet::validate_transaction::<Test>(&tx);

            (res0, res1)
        });

        // The flow of time cannot turn a valid transaction int an invalid one.
        // This is an implication: If a transaction validates at time0, it validates at time1.
        prop_assert!(!res0.is_ok() || res1.is_ok());

        // Check the error message given if the transaction validation fails.
        if let Err(e) = res0 {
            prop_assert_eq!(e, "Time lock restrictions not satisfied");
        }
    }

    #[test]
    fn prop_time_lock_realtime_neighbourhood(before in gen_block_time_real()) {
        let now = RawBlockTime::new(before.as_u64() + 1);
        let after = RawBlockTime::new(now.as_u64() + 1);

        let (res_before, res_now, res_after) = execute_with_alice(|alice| {
            let (utxo0, input0) = tx_input_gen_no_signature();
            let tx = Transaction {
                inputs: vec![input0],
                outputs: vec![TransactionOutput::new_pubkey(ALICE_GENESIS_BALANCE - 50, H256::from(alice))],
                time_lock: now,
            }
            .sign_unchecked(&[utxo0], 0, &alice);

            Timestamp::set_timestamp(before.as_u64() * 1000);
            let res_before = crate::pallet::validate_transaction::<Test>(&tx);

            Timestamp::set_timestamp(now.as_u64() * 1000);
            let res_now = crate::pallet::validate_transaction::<Test>(&tx);

            Timestamp::set_timestamp(after.as_u64() * 1000);
            let res_after = crate::pallet::validate_transaction::<Test>(&tx);

            (res_before, res_now, res_after)
        });

        prop_assert_eq!(res_before, Err("Time lock restrictions not satisfied"));
        prop_assert!(res_now.is_ok());
        prop_assert!(res_after.is_ok());
    }

    #[test]
    fn prop_time_lock_realtime_overflow(
        time in ((u64::MAX / 1000 + 1)..=u64::MAX).prop_map(RawBlockTime::new)
    ) {
        execute_with_alice(|alice| {
            let (utxo0, input0) = tx_input_gen_no_signature();
            let tx = Transaction {
                inputs: vec![input0],
                outputs: vec![TransactionOutput::new_pubkey(ALICE_GENESIS_BALANCE - 50, H256::from(alice))],
                time_lock: time,
            }
            .sign_unchecked(&[utxo0], 0, &alice);

            // Check validate_transaction does not crash due to time lock being close to u64::MAX
            let _ = crate::pallet::validate_transaction::<Test>(&tx);
        });
    }
}

// Testing token creation:
// use crate::tokens::{NftDataHash, TokenId};
use crate::tokens::TokenId;
use rand::Rng;

fn build_random_vec(len: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut vec = Vec::with_capacity(len);
    for _ in 0..len {
        vec.push(rng.gen::<u8>());
    }
    vec
}

#[test]
// Simple creation of tokens
fn test_token_issuance() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();
        let output_new = TransactionOutput {
            value: ALICE_GENESIS_BALANCE,
            destination: Destination::Pubkey(alice_pub_key),
            data: Some(OutputData::TokenIssuanceV1 {
                //token_id: TokenId::new_asset(first_input_hash),
                token_ticker: "BensT".as_bytes().to_vec(),
                amount_to_issue: 1_000_000_000,
                number_of_decimals: 2,
                metadata_uri: "mintlayer.org".as_bytes().to_vec(),
            }),
        };
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![output_new],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);
        let new_utxo_hash = tx.outpoint(0);
        let (_, init_utxo) = genesis_utxo();
        // submit tx - in the test it makes a new UTXO. Checks before that this UTXO has not created yet.
        // After calling `Utxo::spend`, we should check that Storages successfully changed.
        // If it successfully wrote a new UTXO in the Storage, tx goes through all verifications correctly.
        assert!(UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
        assert_ok!(Utxo::spend(Origin::none(), tx));
        assert!(!UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
        // Checking a new UTXO
        assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));

        match UtxoStore::<Test>::get(new_utxo_hash).expect("The new output not found").data {
            Some(OutputData::TokenIssuanceV1 {
                //token_id,
                token_ticker,
                amount_to_issue,
                number_of_decimals,
                metadata_uri,
            }) => {
                //assert_eq!(TokenId::new_asset(first_input_hash), token_id);
                assert_eq!(1_000_000_000, amount_to_issue);
                assert_eq!("BensT".as_bytes().to_vec(), token_ticker);
                assert_eq!(2, number_of_decimals);
                assert_eq!("mintlayer.org".as_bytes().to_vec(), metadata_uri);
            }
            _ => panic!("Transaction data is corrupted"),
        }
    });
}

// todo: This part isn't fully tested, left for the next PR
// #[test]
// // Simple creation of NFT
// fn test_nft_mint() {
//     execute_with_alice(|alice_pub_key| {
//         let (utxo0, input0) = tx_input_gen_no_signature();
//         let first_input_hash = BlakeTwo256::hash(&input0.outpoint.as_ref());
//         let data_hash = NftDataHash::Raw(vec![1, 2, 3, 4, 5]);
//         let output = TransactionOutput {
//             value: 0,
//             destination: Destination::Pubkey(alice_pub_key),
//             data: Some(OutputData::NftMintV1 {
//                 token_id: TokenId::new_asset(first_input_hash),
//                 data_hash: data_hash.clone(),
//                 metadata_uri: "mintlayer.org".as_bytes().to_vec(),
//             }),
//         };
//         let tx = Transaction {
//             inputs: vec![input0],
//             outputs: vec![output],
//             time_lock: Default::default(),
//         }
//         .sign_unchecked(&[utxo0], 0, &alice_pub_key);
//         let new_utxo_hash = tx.outpoint(0);
//         let (_, init_utxo) = genesis_utxo();
//         assert!(UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
//         assert_ok!(Utxo::spend(Origin::none(), tx));
//         assert!(!UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
//         assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
//         assert_eq!(
//             data_hash,
//             UtxoStore::<Test>::get(new_utxo_hash)
//                 .unwrap()
//                 .data
//                 .map(|x| match x {
//                     OutputData::NftMintV1 { data_hash, .. } => data_hash,
//                     _ => NftDataHash::Raw(Vec::new()),
//                 })
//                 .unwrap_or(NftDataHash::Raw(Vec::new()))
//         );
//     })
// }
//
// #[test]
// // NFT might be only unique, we can't create a few nft for one item
// fn test_nft_unique() {
//     execute_with_alice(|alice_pub_key| {
//         let (utxo0, input0) = tx_input_gen_no_signature();
//         let first_input_hash = BlakeTwo256::hash(&input0.outpoint.as_ref());
//
//         let mut nft_data = OutputData::NftMintV1 {
//             token_id: TokenId::new_asset(first_input_hash),
//             data_hash: NftDataHash::Hash32([255; 32]),
//             metadata_uri: "mintlayer.org".as_bytes().to_vec(),
//         };
//         let tx = Transaction {
//             inputs: vec![input0.clone()],
//             outputs: vec![
//                 TransactionOutput {
//                     value: 0,
//                     destination: Destination::Pubkey(alice_pub_key),
//                     data: Some(nft_data.clone()),
//                 },
//                 TransactionOutput::new_pubkey(50, H256::from(alice_pub_key)),
//             ],
//             time_lock: Default::default(),
//         }
//         .sign_unchecked(&[utxo0.clone()], 0, &alice_pub_key);
//         let new_utxo_hash = tx.outpoint(1);
//         let (_, init_utxo) = genesis_utxo();
//         // Submit
//         assert!(UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
//         assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
//         assert!(!UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
//         // Checking a new UTXO
//         assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
//         let new_utxo = tx.outputs[1].clone();
//
//         if let OutputData::NftMintV1 {
//             ref mut token_id, ..
//         } = nft_data
//         {
//             *token_id = TokenId::new_asset(H256::random());
//         }
//         let tx = Transaction {
//             inputs: vec![TransactionInput::new_empty(new_utxo_hash.clone())],
//             outputs: vec![TransactionOutput {
//                 value: 0,
//                 destination: Destination::Pubkey(alice_pub_key),
//                 data: Some(nft_data.clone()),
//             }],
//             time_lock: Default::default(),
//         }
//         .sign_unchecked(&[new_utxo], 0, &alice_pub_key);
//         // Submit
//         assert!(UtxoStore::<Test>::contains_key(H256::from(new_utxo_hash)));
//         frame_support::assert_err_ignore_postinfo!(
//             Utxo::spend(Origin::none(), tx),
//             "digital data has already been minted"
//         );
//     });
// }

// This macro using for the fast creation and sending a tx
macro_rules! test_tx {
    ($data: ident, $checking: tt, $err: expr) => {
        execute_with_alice(|alice_pub_key| {
            let (utxo0, input0) = tx_input_gen_no_signature();
            let output_new = TransactionOutput {
                value: ALICE_GENESIS_BALANCE - 1,
                destination: Destination::Pubkey(alice_pub_key),
                data: Some($data.clone()),
            };
            let tx = Transaction {
                inputs: vec![input0],
                outputs: vec![output_new],
                time_lock: Default::default(),
            }
            .sign_unchecked(&[utxo0], 0, &alice_pub_key);
            let new_utxo_hash = tx.outpoint(0);
            let (_, init_utxo) = genesis_utxo();
            // Send
            assert!(UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
            // We can check what error we are expecting
            if stringify!($checking) == "Err" {
                frame_support::assert_err_ignore_postinfo!(
                    Utxo::spend(Origin::none(), tx),
                    $err
                );
                assert!(UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
                assert!(!UtxoStore::<Test>::contains_key(new_utxo_hash));
            } else if stringify!($checking) == "Ok" {
                // We can check is that success
                assert_ok!(Utxo::spend(Origin::none(), tx));
                assert!(!UtxoStore::<Test>::contains_key(H256::from(init_utxo)));
                assert!(UtxoStore::<Test>::contains_key(new_utxo_hash));
            }
        });
    };
}

#[test]
fn test_tokens_issuance_empty_ticker() {
    // Ticker empty
    let data = OutputData::TokenIssuanceV1 {
        token_ticker: vec![],
        amount_to_issue: 1_000_000_000,
        number_of_decimals: 2,
        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
    };
    test_tx!(data, Err, "token ticker can't be empty");
}

#[test]
fn test_tokens_issuance_too_big_ticker() {
    // Ticker too long
    let data = OutputData::TokenIssuanceV1 {
        token_ticker: Vec::from([b"A"[0]; 10_000]),
        amount_to_issue: 1_000_000_000,
        number_of_decimals: 2,
        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
    };
    test_tx!(data, Err, "token ticker is too long");
}

#[test]
fn test_tokens_issuance_amount_zero() {
    // Amount to issue is zero
    let data = OutputData::TokenIssuanceV1 {
        token_ticker: b"BensT".to_vec(),
        amount_to_issue: 0,
        number_of_decimals: 2,
        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
    };
    test_tx!(data, Err, "output value must be nonzero");
}

#[test]
fn test_tokens_issuance_too_big_decimals() {
    // Number of decimals more than 18 numbers
    let data = OutputData::TokenIssuanceV1 {
        token_ticker: b"BensT".to_vec(),
        amount_to_issue: 1_000_000_000,
        number_of_decimals: 19,
        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
    };
    test_tx!(data, Err, "too long decimals");
}

#[test]
fn test_tokens_issuance_empty_metadata() {
    // metadata_uri empty
    let data = OutputData::TokenIssuanceV1 {
        token_ticker: b"BensT".to_vec(),
        amount_to_issue: 1_000_000_000,
        number_of_decimals: 18,
        metadata_uri: vec![],
    };
    test_tx!(data, Ok, "");
}

#[test]
fn test_tokens_issuance_too_long_metadata() {
    // metadata_uri too long
    let data = OutputData::TokenIssuanceV1 {
        token_ticker: b"BensT".to_vec(),
        amount_to_issue: 1_000_000_000,
        number_of_decimals: 18,
        metadata_uri: Vec::from([0u8; 10_000]),
    };
    test_tx!(data, Err, "token metadata uri is too long");
}

#[test]
fn test_tokens_issuance_with_junk_data() {
    // The data field of the maximum allowed length filled with random garbage
    let mut rng = rand::thread_rng();
    let garbage = build_random_vec(100);
    let data = OutputData::TokenIssuanceV1 {
        token_ticker: vec![0, 255, 254, 2, 1],
        amount_to_issue: rng.gen::<u64>() as u128,
        number_of_decimals: 18,
        metadata_uri: garbage.clone(),
    };
    test_tx!(data, Err, "token ticker has none ascii characters");
}

#[test]
fn test_tokens_issuance_with_corrupted_uri() {
    let mut rng = rand::thread_rng();
    let garbage = build_random_vec(100);
    // garbage uri
    let data = OutputData::TokenIssuanceV1 {
        token_ticker: b"BensT".to_vec(),
        amount_to_issue: rng.gen::<u64>() as u128,
        number_of_decimals: 18,
        metadata_uri: garbage,
    };
    test_tx!(data, Err, "metadata uri has none ascii characters");
}

#[test]
fn test_two_token_creation_in_one_tx() {
    execute_with_alice(|alice_pub_key| {
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(alice_pub_key),
                    OutputData::TokenIssuanceV1 {
                        token_ticker: b"Enric".to_vec(),
                        amount_to_issue: 1_000_000_000,
                        number_of_decimals: 2,
                        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(alice_pub_key),
                    OutputData::TokenIssuanceV1 {
                        token_ticker: b"Ben".to_vec(),
                        amount_to_issue: 2_000_000_000,
                        number_of_decimals: 3,
                        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0], 0, &alice_pub_key);
        frame_support::assert_err_ignore_postinfo!(
            Utxo::spend(Origin::none(), tx),
            "this id can't be used for a new token"
        );
    });
}

// Let's wrap common acts
fn test_tx_issuance_for_transfer<F>(expecting_err_msg: &'static str, test_func: F)
where
    F: Fn(TokenId, Public, Public, H256, TransactionOutput<H256>) -> Transaction<H256>,
{
    let (mut test_ext, alice_pub_key, karl_pub_key) = alice_test_ext_and_keys();
    test_ext.execute_with(|| {
        // Alice issue 1_000_000_000 MLS-01, and send them to Karl
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0.clone()],
            outputs: vec![
                TransactionOutput::new_pubkey(
                    ALICE_GENESIS_BALANCE - 90,
                    H256::from(alice_pub_key),
                ),
                TransactionOutput::new_p2pk_with_data(
                    10,
                    H256::from(karl_pub_key),
                    OutputData::TokenIssuanceV1 {
                        token_ticker: "BensT".as_bytes().to_vec(),
                        amount_to_issue: 1_000_000_000,
                        // Should be not more than 18 numbers
                        number_of_decimals: 2,
                        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0.clone()], 0, &alice_pub_key);
        let token_id = TokenId::new(&tx.inputs[0]);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));

        let token_utxo_hash = tx.outpoint(1);
        let token_utxo: TransactionOutput<H256> = tx.outputs[1].clone();
        // Call a test func
        let tx = test_func(
            token_id,
            alice_pub_key,
            karl_pub_key,
            token_utxo_hash,
            token_utxo,
        );
        frame_support::assert_err_ignore_postinfo!(
            Utxo::spend(Origin::none(), tx),
            expecting_err_msg
        );
    });
}

#[test]
fn test_token_transfer_with_wrong_token_id() {
    let test_fun = Box::new(
        move |_token_id,
              alice_pub_key,
              karl_pub_key,
              token_utxo_hash,
              token_utxo: TransactionOutput<H256>| {
            let input = TransactionInput::new_empty(token_utxo_hash);
            Transaction {
                inputs: vec![input.clone()],
                outputs: vec![TransactionOutput::new_p2pk_with_data(
                    ALICE_GENESIS_BALANCE - u64::MAX as Value,
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: TokenId::new(&input),
                        amount: 100_000_000,
                    },
                )],
                time_lock: Default::default(),
            }
            .sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key)
        },
    );
    test_tx_issuance_for_transfer("input for the token not found", test_fun);
}

#[test]
fn test_token_transfer_exceed_amount_tokens() {
    let test_fun = Box::new(
        move |token_id,
              alice_pub_key,
              karl_pub_key,
              token_utxo_hash,
              token_utxo: TransactionOutput<H256>| {
            Transaction {
                inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
                outputs: vec![TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id,
                        amount: 1_000_000_001,
                    },
                )],
                time_lock: Default::default(),
            }
            .sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key)
        },
    );
    test_tx_issuance_for_transfer("output value must not exceed input value", test_fun);
}

#[test]
fn test_token_transfer_exceed_amount_mlt() {
    let test_fun = Box::new(
        move |token_id: TokenId,
              alice_pub_key,
              karl_pub_key,
              token_utxo_hash,
              token_utxo: TransactionOutput<H256>| {
            Transaction {
                inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
                outputs: vec![TransactionOutput::new_p2pk_with_data(
                    1_000_000_000,
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: token_id.clone(),
                        amount: 1_000_000_000,
                    },
                )],
                time_lock: Default::default(),
            }
            .sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key)
        },
    );
    test_tx_issuance_for_transfer("output value must not exceed input value", test_fun);
}

#[test]
fn test_token_transfer_send_part_others_burn() {
    let test_fun = Box::new(
        move |token_id: TokenId,
              alice_pub_key,
              karl_pub_key,
              token_utxo_hash,
              token_utxo: TransactionOutput<H256>| {
            Transaction {
                inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
                outputs: vec![
                    // Send only 30%, let's forget about another 70% of tokens
                    TransactionOutput::new_p2pk_with_data(
                        0,
                        H256::from(alice_pub_key),
                        OutputData::TokenTransferV1 {
                            token_id: token_id.clone(),
                            amount: 300_000_000,
                        },
                    ),
                ],
                time_lock: Default::default(),
            }
            .sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key)
        },
    );
    test_tx_issuance_for_transfer("output value must not exceed input value", test_fun);
}

#[test]
fn test_token_transfer() {
    let (mut test_ext, alice_pub_key, karl_pub_key) = alice_test_ext_and_keys();
    test_ext.execute_with(|| {
        // Alice issue 1_000_000_000 MLS-01, and send them to Karl
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![
                TransactionOutput::new_pubkey(
                    ALICE_GENESIS_BALANCE - 90,
                    H256::from(alice_pub_key),
                ),
                TransactionOutput::new_p2pk_with_data(
                    90,
                    H256::from(karl_pub_key),
                    OutputData::TokenIssuanceV1 {
                        token_ticker: "BensT".as_bytes().to_vec(),
                        amount_to_issue: 1_000_000_000,
                        // Should be not more than 18 numbers
                        number_of_decimals: 2,
                        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0.clone()], 0, &alice_pub_key);
        let token_id = TokenId::new(&tx.inputs[0]);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let token_utxo_hash = tx.outpoint(1);
        let token_utxo = tx.outputs[1].clone();

        // Let's send 300_000_000 and rest back
        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
            outputs: vec![
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: token_id.clone(),
                        amount: 300_000_000,
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(karl_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: token_id.clone(),
                        amount: 700_000_000,
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let alice_tokens_utxo_hash = tx.outpoint(0);
        let karl_tokens_utxo_hash = tx.outpoint(1);
        let karl_tokens_utxo = tx.outputs[1].clone();
        assert!(!UtxoStore::<Test>::contains_key(H256::from(
            token_utxo_hash
        )));
        assert!(UtxoStore::<Test>::contains_key(alice_tokens_utxo_hash));
        assert!(UtxoStore::<Test>::contains_key(karl_tokens_utxo_hash));

        // should be success
        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(karl_tokens_utxo_hash)],
            outputs: vec![
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: token_id.clone(),
                        amount: 400_000_000,
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(karl_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: token_id.clone(),
                        amount: 300_000_000,
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[karl_tokens_utxo], 0, &karl_pub_key);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        assert_eq!(
            300_000_000,
            UtxoStore::<Test>::get(alice_tokens_utxo_hash)
                .unwrap()
                .data
                .map(|x| match x {
                    OutputData::TokenTransferV1 { amount, .. } => amount,
                    _ => 0,
                })
                .unwrap_or(0)
        );

        let new_alice_tokens_utxo_hash = tx.outpoint(0);
        assert!(UtxoStore::<Test>::contains_key(new_alice_tokens_utxo_hash));
        assert_eq!(
            400_000_000,
            UtxoStore::<Test>::get(new_alice_tokens_utxo_hash)
                .unwrap()
                .data
                .map(|x| match x {
                    OutputData::TokenTransferV1 { amount, .. } => amount,
                    _ => 0,
                })
                .unwrap_or(0)
        );
    });
}

// todo: This part isn't fully tested, left for the next PR
// #[test]
// fn test_nft_transferring() {
//     let (mut test_ext, alice_pub_key, karl_pub_key) = new_test_ext_and_keys();
//     test_ext.execute_with(|| {
//         let token_id = TokenId::new_asset(H256::random());
//         // Alice issue 1000 MLS-01, and send them to Karl and the rest back to herself
//         let (utxo0, input0) = tx_input_gen_no_signature();
//         let data_hash = NftDataHash::Raw(build_random_vec(32));
//         let tx = Transaction {
//             inputs: vec![input0],
//             outputs: vec![
//                 TransactionOutput::new_pubkey(90, H256::from(alice_pub_key)),
//                 TransactionOutput::new_p2pk_with_data(
//                     10,
//                     H256::from(karl_pub_key),
//                     OutputData::NftMintV1 {
//                         token_id: token_id.clone(),
//                         data_hash: data_hash.clone(),
//                         metadata_uri: "mintlayer.org".as_bytes().to_vec(),
//                     },
//                 ),
//             ],
//             time_lock: Default::default(),
//         }
//         .sign_unchecked(&[utxo0.clone()], 0, &alice_pub_key);
//         assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
//         let token_utxo_hash = tx.outpoint(1);
//         let token_utxo = tx.outputs[1].clone();
//
//         // Let's fail on wrong token id
//         let tx = Transaction {
//             inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
//             outputs: vec![TransactionOutput::new_p2pk_with_data(
//                 0,
//                 H256::from(alice_pub_key),
//                 OutputData::TokenTransferV1 {
//                     token_id: TokenId::new_asset(H256::random()),
//                     amount: 1_00_000_000,
//                 },
//             )],
//             time_lock: Default::default(),
//         }
//         .sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key);
//         frame_support::assert_err_ignore_postinfo!(
//             Utxo::spend(Origin::none(), tx),
//             "input for the token not found"
//         );
//         // Let's fail on exceed token amount
//         let tx = Transaction {
//             inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
//             outputs: vec![TransactionOutput::new_p2pk_with_data(
//                 0,
//                 H256::from(alice_pub_key),
//                 OutputData::TokenTransferV1 {
//                     token_id: token_id.clone(),
//                     amount: 1_000_000_001,
//                 },
//             )],
//             time_lock: Default::default(),
//         }
//         .sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key);
//         frame_support::assert_err_ignore_postinfo!(
//             Utxo::spend(Origin::none(), tx),
//             "output value must not exceed input value"
//         );
//
//         // Let's send a big amount of MLT with the correct tokens
//         let tx = Transaction {
//             inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
//             outputs: vec![TransactionOutput::new_p2pk_with_data(
//                 1_000_000_000,
//                 H256::from(alice_pub_key),
//                 OutputData::TokenTransferV1 {
//                     token_id: token_id.clone(),
//                     amount: 1_000_000_000,
//                 },
//             )],
//             time_lock: Default::default(),
//         }
//         .sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key);
//         frame_support::assert_err_ignore_postinfo!(
//             Utxo::spend(Origin::none(), tx),
//             "output value must not exceed input value"
//         );
//
//         // should be success
//         let tx = Transaction {
//             inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
//             outputs: vec![TransactionOutput::new_p2pk_with_data(
//                 0,
//                 H256::from(alice_pub_key),
//                 OutputData::TokenTransferV1 {
//                     token_id: token_id.clone(),
//                     amount: 1,
//                 },
//             )],
//             time_lock: Default::default(),
//         }
//         .sign_unchecked(&[token_utxo], 0, &karl_pub_key);
//         assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
//         let nft_utxo_hash = tx.outpoint(0);
//         assert!(!UtxoStore::<Test>::contains_key(H256::from(
//             token_utxo_hash
//         )));
//         assert!(UtxoStore::<Test>::contains_key(nft_utxo_hash));
//         assert_eq!(
//             data_hash,
//             crate::get_output_by_token_id::<Test>(token_id.clone())
//                 .unwrap()
//                 .data
//                 .map(|x| match x {
//                     OutputData::NftMintV1 { data_hash, .. } => data_hash,
//                     _ => NftDataHash::Raw(Vec::new()),
//                 })
//                 .unwrap_or(NftDataHash::Raw(Vec::new()))
//         );
//     });
// }

#[test]
// Test tx where Input with token and without MLT, output has token (without MLT)
fn test_token_creation_with_insufficient_fee() {
    let (mut test_ext, alice_pub_key, karl_pub_key) = alice_test_ext_and_keys();
    test_ext.execute_with(|| {
        // Alice issue 1000 MLS-01, and send them to Karl and the rest back to herself
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![
                TransactionOutput::new_pubkey(
                    ALICE_GENESIS_BALANCE - u64::MAX as Value,
                    H256::from(karl_pub_key),
                ),
                TransactionOutput::new_p2pk_with_data(
                    crate::tokens::Mlt(99).to_munit(),
                    H256::from(karl_pub_key),
                    OutputData::TokenIssuanceV1 {
                        token_ticker: "BensT".as_bytes().to_vec(),
                        amount_to_issue: 1_000_000_000,
                        number_of_decimals: 2,
                        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0.clone()], 0, &alice_pub_key);

        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let token_utxo_hash = tx.outpoint(1);
        let token_utxo = tx.outputs[1].clone();
        let tx = Transaction {
            inputs: vec![
                // Use here token issuance for example
                TransactionInput::new_empty(token_utxo_hash),
            ],
            outputs: vec![TransactionOutput::new_p2pk_with_data(
                0,
                H256::from(karl_pub_key),
                OutputData::TokenIssuanceV1 {
                    token_ticker: b"Enric".to_vec(),
                    amount_to_issue: 1_000_000_000,
                    number_of_decimals: 2,
                    metadata_uri: "mintlayer.org".as_bytes().to_vec(),
                },
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[token_utxo], 0, &karl_pub_key);
        frame_support::assert_err_ignore_postinfo!(
            Utxo::spend(Origin::none(), tx),
            "insufficient fee"
        );
    });
}

#[test]
fn test_transfer_and_issuance_in_one_tx() {
    let (mut test_ext, alice_pub_key, karl_pub_key) = alice_test_ext_and_keys();
    test_ext.execute_with(|| {
        // Alice issue 1_000_000_000 MLS-01, and send them to Karl
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![
                TransactionOutput::new_pubkey(
                    ALICE_GENESIS_BALANCE - crate::tokens::Mlt(1000).to_munit(),
                    H256::from(alice_pub_key),
                ),
                TransactionOutput::new_p2pk_with_data(
                    crate::tokens::Mlt(1000).to_munit(),
                    H256::from(karl_pub_key),
                    OutputData::TokenIssuanceV1 {
                        token_ticker: "BensT".as_bytes().to_vec(),
                        amount_to_issue: 1_000_000_000,
                        number_of_decimals: 2,
                        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0.clone()], 0, &alice_pub_key);
        let first_issuance_token_id = TokenId::new(&tx.inputs[0]);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let token_utxo_hash = tx.outpoint(1);
        let token_utxo = tx.outputs[1].clone();

        // Let's send 300_000_000 and rest back and create another token
        let tx = Transaction {
            inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
            outputs: vec![
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: first_issuance_token_id.clone(),
                        amount: 300_000_000,
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(karl_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: first_issuance_token_id.clone(),
                        amount: 700_000_000,
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(karl_pub_key),
                    OutputData::TokenIssuanceV1 {
                        token_ticker: "Token".as_bytes().to_vec(),
                        amount_to_issue: 5_000_000_000,
                        // Should be not more than 18 numbers
                        number_of_decimals: 12,
                        metadata_uri: "mintlayer.org".as_bytes().to_vec(),
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let alice_transfer_utxo_hash = tx.outpoint(0);
        let karl_transfer_utxo_hash = tx.outpoint(1);
        let karl_issuance_utxo_hash = tx.outpoint(2);
        assert!(!UtxoStore::<Test>::contains_key(H256::from(
            token_utxo_hash
        )));
        assert!(UtxoStore::<Test>::contains_key(alice_transfer_utxo_hash));
        assert!(UtxoStore::<Test>::contains_key(karl_transfer_utxo_hash));
        assert!(UtxoStore::<Test>::contains_key(karl_issuance_utxo_hash));

        // Let's check token transfer
        UtxoStore::<Test>::get(alice_transfer_utxo_hash)
            .unwrap()
            .data
            .map(|x| match x {
                OutputData::TokenTransferV1 { token_id, amount } => {
                    assert_eq!(token_id, first_issuance_token_id);
                    assert_eq!(amount, 300_000_000);
                }
                _ => {
                    panic!("corrupted data");
                }
            })
            .unwrap();

        UtxoStore::<Test>::get(karl_transfer_utxo_hash)
            .unwrap()
            .data
            .map(|x| match x {
                OutputData::TokenTransferV1 { token_id, amount } => {
                    assert_eq!(token_id, first_issuance_token_id);
                    assert_eq!(amount, 700_000_000);
                }
                _ => {
                    panic!("corrupted data");
                }
            })
            .unwrap();

        // Let's check token issuance
        UtxoStore::<Test>::get(karl_issuance_utxo_hash)
            .unwrap()
            .data
            .map(|x| match x {
                OutputData::TokenIssuanceV1 {
                    token_ticker,
                    amount_to_issue,
                    number_of_decimals,
                    metadata_uri,
                } => {
                    assert_eq!(token_ticker, "Token".as_bytes().to_vec());
                    assert_eq!(amount_to_issue, 5_000_000_000);
                    assert_eq!(number_of_decimals, 12);
                    assert_eq!(metadata_uri, "mintlayer.org".as_bytes().to_vec());
                }
                _ => {
                    panic!("corrupted data");
                }
            })
            .unwrap();
    });
}

#[test]
fn test_transfer_for_multiple_tokens() {
    let (mut test_ext, alice_pub_key, karl_pub_key) = alice_test_ext_and_keys();
    test_ext.execute_with(|| {
        //
        // Issue token 1 and send all tokens to Karl
        //
        let (utxo0, input0) = tx_input_gen_no_signature();
        let tx = Transaction {
            inputs: vec![input0],
            outputs: vec![TransactionOutput::new_p2pk_with_data(
                ALICE_GENESIS_BALANCE - crate::tokens::Mlt(100).to_munit(),
                H256::from(karl_pub_key),
                OutputData::TokenIssuanceV1 {
                    token_ticker: "TKN1".as_bytes().to_vec(),
                    amount_to_issue: 1_000_000_000,
                    number_of_decimals: 2,
                    metadata_uri: "tkn1.mintlayer.org".as_bytes().to_vec(),
                },
            )],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[utxo0.clone()], 0, &alice_pub_key);
        let tkn1_token_id = TokenId::new(&tx.inputs[0]);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let tkn1_utxo_hash = tx.outpoint(0);
        let tkn1_utxo = tx.outputs[0].clone();
        //
        // Issue token 2 and send all tokens to Alice
        //
        let input1 = TransactionInput::new_empty(tkn1_utxo_hash);
        let tx = Transaction {
            inputs: vec![input1],
            outputs: vec![
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: tkn1_token_id.clone(),
                        amount: 1_000_000_000,
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    ALICE_GENESIS_BALANCE - crate::tokens::Mlt(100).to_munit(),
                    H256::from(alice_pub_key),
                    OutputData::TokenIssuanceV1 {
                        token_ticker: "TKN2".as_bytes().to_vec(),
                        amount_to_issue: 2_000_000_000,
                        number_of_decimals: 4,
                        metadata_uri: "tkn2.mintlayer.org".as_bytes().to_vec(),
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&[tkn1_utxo.clone()], 0, &karl_pub_key);
        let tkn2_token_id = TokenId::new(&tx.inputs[0]);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let tkn1_utxo_hash = tx.outpoint(0);
        let tkn2_utxo_hash = tx.outpoint(1);
        //
        // Issue token 3 and send all tokens to Karl
        //
        let input1 = TransactionInput::new_empty(tkn1_utxo_hash);
        let input2 = TransactionInput::new_empty(tkn2_utxo_hash);
        let prev_utxos = [tx.outputs[0].clone(), tx.outputs[1].clone()];
        let tx = Transaction {
            inputs: vec![input1, input2],
            outputs: vec![
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(karl_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: tkn1_token_id.clone(),
                        amount: 1_000_000_000,
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(karl_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: tkn2_token_id.clone(),
                        amount: 2_000_000_000,
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    ALICE_GENESIS_BALANCE - crate::tokens::Mlt(100).to_munit(),
                    H256::from(karl_pub_key),
                    OutputData::TokenIssuanceV1 {
                        token_ticker: "TKN3".as_bytes().to_vec(),
                        amount_to_issue: 3_000_000_000,
                        number_of_decimals: 6,
                        metadata_uri: "tkn3.mintlayer.org".as_bytes().to_vec(),
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&prev_utxos, 0, &alice_pub_key)
        .sign_unchecked(&prev_utxos, 1, &alice_pub_key);
        let tkn3_token_id = TokenId::new(&tx.inputs[0]);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let tkn1_utxo_hash = tx.outpoint(0);
        let tkn2_utxo_hash = tx.outpoint(1);
        let tkn3_utxo_hash = tx.outpoint(2);

        //
        // Transfer 3 kinds of tokens to Alice and check them all
        //
        let input1 = TransactionInput::new_empty(tkn1_utxo_hash);
        let input2 = TransactionInput::new_empty(tkn2_utxo_hash);
        let input3 = TransactionInput::new_empty(tkn3_utxo_hash);
        let prev_utxos = [tx.outputs[0].clone(), tx.outputs[1].clone(), tx.outputs[2].clone()];
        let tx = Transaction {
            inputs: vec![input1, input2, input3],
            outputs: vec![
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: tkn1_token_id.clone(),
                        amount: 1_000_000_000,
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    0,
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: tkn2_token_id.clone(),
                        amount: 2_000_000_000,
                    },
                ),
                TransactionOutput::new_p2pk_with_data(
                    ALICE_GENESIS_BALANCE - crate::tokens::Mlt(100).to_munit(),
                    H256::from(alice_pub_key),
                    OutputData::TokenTransferV1 {
                        token_id: tkn3_token_id.clone(),
                        amount: 3_000_000_000,
                    },
                ),
            ],
            time_lock: Default::default(),
        }
        .sign_unchecked(&prev_utxos, 0, &karl_pub_key)
        .sign_unchecked(&prev_utxos, 1, &karl_pub_key)
        .sign_unchecked(&prev_utxos, 2, &karl_pub_key);
        assert_ok!(Utxo::spend(Origin::none(), tx.clone()));
        let tkn1_utxo_hash = tx.outpoint(0);
        let tkn2_utxo_hash = tx.outpoint(1);
        let tkn3_utxo_hash = tx.outpoint(2);
        // Check tkn1
        UtxoStore::<Test>::get(tkn1_utxo_hash)
            .unwrap()
            .data
            .map(|x| match x {
                OutputData::TokenTransferV1 { token_id, amount } => {
                    assert_eq!(token_id, tkn1_token_id);
                    assert_eq!(amount, 1_000_000_000);
                }
                _ => {
                    panic!("corrupted data");
                }
            })
            .unwrap();
        // Check tkn2
        UtxoStore::<Test>::get(tkn2_utxo_hash)
            .unwrap()
            .data
            .map(|x| match x {
                OutputData::TokenTransferV1 { token_id, amount } => {
                    assert_eq!(token_id, tkn2_token_id);
                    assert_eq!(amount, 2_000_000_000);
                }
                _ => {
                    panic!("corrupted data");
                }
            })
            .unwrap();
        // Check tkn3
        UtxoStore::<Test>::get(tkn3_utxo_hash)
            .unwrap()
            .data
            .map(|x| match x {
                OutputData::TokenTransferV1 { token_id, amount } => {
                    assert_eq!(token_id, tkn3_token_id);
                    assert_eq!(amount, 3_000_000_000);
                }
                _ => {
                    panic!("corrupted data");
                }
            })
            .unwrap();
    });
}

#[test]
fn test_immutable_tx_format() {
    // todo: Testing the compatibility of the old version with the new one - not done yet
}

#[test]
fn test_burn_tokens() {
    // todo: Burn tokens has not tested yet
}

#[test]
fn test_token_id() {
    // todo: Testing token id - not done yet
}
