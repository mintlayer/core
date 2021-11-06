#!/usr/bin/env python3
# Copyright (c) 2021 RBB S.r.l
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.
"""Smart contract test

"""
# Imports should be in PEP8 ordering (std library first, then third party
# libraries then local imports).
from collections import defaultdict

from substrateinterface import Keypair
import test_framework.mintlayer.utxo as utxo
import test_framework.mintlayer.contract as contract

from test_framework.test_framework import MintlayerTestFramework
from test_framework.util import (
    assert_equal,
    connect_nodes,
    wait_until,
)

import os

# helper function to reduce code duplication
def submit_pp_tx(client, input_utxo, alice, value, outputs):
    outputs.insert(0, utxo.Output(
        value=value,
        header=0,
        destination=utxo.DestPubkey(alice.public_key)
    ))
    tx = utxo.Transaction(
        client,
        inputs=[
            utxo.Input(input_utxo.outpoint(0)),
        ],
        outputs = outputs,
    ).sign(alice, [input_utxo.outputs[0]], [0])
    return tx, client.submit(alice, tx)

class ExampleTest(MintlayerTestFramework):
    # Each functional test is a subclass of the MintlayerTestFramework class.

    # Override the set_test_params(), add_options(), setup_chain(), setup_network()
    # and setup_nodes() methods to customize the test setup as required.

    def set_test_params(self):
        """Override test parameters for your individual test.

        This method must be overridden and num_nodes must be exlicitly set."""
        self.setup_clean_chain = True
        self.num_nodes = 3
        # Use self.extra_args to change command-line arguments for the nodes
        self.extra_args = [[], [], []]

        # self.log.info("I've finished set_test_params")  # Oops! Can't run self.log before run_test()

    def setup_network(self):
        """Setup the test network topology

        Often you won't need to override this, since the standard network topology
        (linear: node0 <-> node1 <-> node2 <-> ...) is fine for most tests.

        If you do override this method, remember to start the nodes, assign
        them to self.nodes, connect them and then sync."""

        self.setup_nodes()

        # In this test, we're not connecting node2 to node0 or node1. Calls to
        # sync_all() should not include node2, since we're not expecting it to
        # sync.
        connect_nodes(self.nodes[0], self.nodes[1])

    def run_test(self):
        """Main test logic"""
        client = self.nodes[0].rpc_client
        substrate = client.substrate
        alice = Keypair.create_from_uri('//Alice')
        bob = Keypair.create_from_uri('//Bob')

        initial_utxo = [x for x in client.utxos_for(alice) if x[1].value >= 50][0]
        value = 10000000000

        tx = utxo.Transaction(
            client,
            inputs=[
                utxo.Input(initial_utxo[0]),
            ],
            outputs=[
                utxo.Output(
                    value=value,
                    header=0,
                    destination=utxo.DestPubkey(alice.public_key)
                ),
            ]
        ).sign(alice, [initial_utxo[1]])
        (ext, blk) = client.submit(alice, tx)

        # invalid bytecode
        value -= 1

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [utxo.Output(
            value=1,
            header=0,
            destination=utxo.DestCreatePP(
                code=[0x00],
                data=[0xed, 0x4b, 0x9d, 0x1b],
            )
        )])
        assert_equal(contract.getContractAddresses(substrate, blk), None)

        # invalid value
        (invalid_tx, res) = submit_pp_tx(client, tx, alice, value, [utxo.Output(
            value=0,
            header=0,
            destination=utxo.DestCreatePP(
                code=os.path.join(os.path.dirname(__file__), "assets/pooltester.wasm"),
                data=[0xed, 0x4b, 0x9d, 0x1b],
            )
        )])
        assert_equal(res, None)

        # valid data
        value -= 1

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [utxo.Output(
            value=1,
            header=0,
            destination=utxo.DestCreatePP(
                code=os.path.join(os.path.dirname(__file__), "assets/pooltester.wasm"),
                data=[0x9b, 0xae, 0x9d, 0x5e],
            )
        )])

        (ss58, acc_id) = contract.getContractAddresses(substrate, blk)
        contractInstance = contract.ContractInstance(
            ss58,
            os.path.join(os.path.dirname(__file__), "assets/pooltester.json"),
            substrate
        )

        # verify the initial state of the smart contract
        result = contractInstance.read(alice, "get")
        assert_equal(result.contract_result_data.value, 1337)

        # valid contract call
        value -= 1
        msg_data = contractInstance.generate_message_data("flip", {})

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [utxo.Output(
            value=1,
            header=0,
            destination=utxo.DestCallPP(
                dest_account=acc_id,
                input_data=bytes.fromhex(msg_data.to_hex()[2:]),
            )
        )])
        result = contractInstance.read(alice, "get")
        assert_equal(result.contract_result_data.value, -1337)

        # invalid `value` given
        msg_data = contractInstance.generate_message_data("flip", {})

        (invalid_tx, res) = submit_pp_tx(client, tx, alice, value, [utxo.Output(
            value=0,
            header=0,
            destination=utxo.DestCallPP(
                dest_account=alice.public_key,
                input_data=bytes.fromhex(msg_data.to_hex()[2:]),
            )
        )])
        assert_equal(res, None)

        # query the initial value of the contract
        #
        # each successful tranfser will update the value by one
        # and each call that fails doesn't change the value
        result = contractInstance.read(alice, "get")
        assert_equal(result.contract_result_data.value, -1337)

        # try to call contract without funding it
        msg_data = contractInstance.generate_message_data("send_to_pubkey", { "dest": bob.public_key, "value": 555 })
        value -= 555

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [utxo.Output(
            value = 555,
            header = 0,
            destination = utxo.DestCallPP(
                dest_account = acc_id,
                input_data = bytes.fromhex(msg_data.to_hex()[2:]),
            )
        )])

        # call failed, the value is not updated
        # assert_equal(get_state_var(contractInstance, client, alice), -1337)

        # fund the contract (but not enough) and call it
        msg_data = contractInstance.generate_message_data("send_to_pubkey", { "dest": bob.public_key, "value": 500 })
        value -= 500

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [
            utxo.Output(
                value = 400,
                header = 0,
                destination = utxo.DestFundPP(acc_id)
            ),
            utxo.Output(
                value = 100,
                header = 0,
                destination = utxo.DestCallPP(
                    dest_account = acc_id,
                    input_data = bytes.fromhex(msg_data.to_hex()[2:]),
                )
            ),
        ])

        # call failed, the value is not updated
        result = contractInstance.read(alice, "get")
        assert_equal(result.contract_result_data.value, -1337)

        """ Fund the contract and call it """
        msg_data = contractInstance.generate_message_data("send_to_pubkey", { "dest": bob.public_key, "value": 500 })
        value -= 200

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [
            utxo.Output(
                value = 100,
                header = 0,
                destination = utxo.DestFundPP(acc_id)
            ),
            utxo.Output(
                value = 100,
                header = 0,
                destination = utxo.DestCallPP(
                    dest_account = acc_id,
                    input_data = bytes.fromhex(msg_data.to_hex()[2:]),
                )
            ),
        ])

        # call succeeded, the value is updated
        result = contractInstance.read(alice, "get")
        assert_equal(result.contract_result_data.value, -1336)

        # verify that Bob has 1 UTXO with value 500
        bobs = [x for x in client.utxos_for(bob.public_key)]
        assert_equal(len(bobs), 1)
        assert_equal(bobs[0][1].json()["value"], 500)

        # verify that the contract only has CallPP UTXOs
        contract_utxos = [x for x in client.utxos_for(acc_id[2:])]
        callpp_utxos = [x for x in contract_utxos if list(x[1].json()["destination"])[0] == "CallPP"]
        assert_equal(len(contract_utxos), len(callpp_utxos))

        """ Fund the contract and call it but don't transfer all of the funds """
        msg_data = contractInstance.generate_message_data("send_to_pubkey", { "dest": bob.public_key, "value": 200 })
        value -= 600

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [
            utxo.Output(
                value = 500,
                header = 0,
                destination = utxo.DestFundPP(acc_id)
            ),
            utxo.Output(
                value = 100,
                header = 0,
                destination = utxo.DestCallPP(
                    dest_account = acc_id,
                    input_data = bytes.fromhex(msg_data.to_hex()[2:]),
                )
            ),
        ])

        # verify that bob has two UTXOs and that their total value is 700
        bobs = [x for x in client.utxos_for(bob.public_key)]
        total_value = sum([x[1].json()["value"] for x in bobs])
        assert_equal(len(bobs), 2)
        assert_equal(total_value, 700)

        # verify that the contract has one FundPP UTXO with value 300
        fundpps = [x for x in client.utxos_for(acc_id[2:]) if list(x[1].json()["destination"])[0] == "FundPP"]
        assert_equal(len(fundpps), 1)
        assert_equal(fundpps[0][1].json()["value"], 300)

        # try to call a contract that doesn't exist (alice's public key
        # doesn't point to a valid smart contract)
        #
        # TODO: because we don't have gas refunding, the money is still
        # spent, i.e., if the UTXO set is queried, you'll find a UTXO
        # with value 888 meaning user just lost his money which is
        # not the correct behavior but the implementation is still under way
        value -= 888

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [utxo.Output(
            value = 888,
            header = 0,
            destination = utxo.DestCallPP(
                dest_account = alice.public_key,
                input_data = [0x00, 0x01, 0x02, 0x03],
            )
        )])

        result = contractInstance.read(alice, "get")
        assert_equal(result.contract_result_data.value, -1335)

        # Test cross-contract calls
        #
        # First instantiate another smart contract and verify it has
        # been created correctly by querying its value.
        #
        # Then call the `set_value()` method of newly instantiated contract
        # indirectly by creating a UTXO that calls the pooltester's
        # `call_contract()` method which dispatches the call to `set_value()`
        #
        # When all that's done, query the value again and verify that it has been updated
        value -= 111

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [utxo.Output(
            value = 111,
            header = 0,
            destination = utxo.DestCreatePP(
                code = os.path.join(os.path.dirname(__file__), "assets/c2c_tester.wasm"),
                data = [0xed, 0x4b, 0x9d, 0x1b],
            )
        )])

        (ss58_c2c, acc_id_c2c) = contract.getContractAddresses(substrate, blk)
        c2cInstance = contract.ContractInstance(
            ss58_c2c,
            os.path.join(os.path.dirname(__file__), "assets/c2c_tester.json"),
            substrate
        )

        # verify the initial state of the smart contract
        result = c2cInstance.read(alice, "get")
        assert_equal(result.contract_result_data.value, 555)

        msg_data = contractInstance.generate_message_data("call_contract", {
            "dest": acc_id_c2c,
            "selector": "0xc6298215",
            "value": 999,
        })
        value -= 600

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [
            utxo.Output(
                value = 500,
                header = 0,
                destination = utxo.DestFundPP(acc_id)
            ),
            utxo.Output(
                value = 100,
                header = 0,
                destination = utxo.DestCallPP(
                    dest_account = acc_id,
                    input_data = bytes.fromhex(msg_data.to_hex()[2:]),
                )
            )
        ])

        # verify that the call succeeded
        result = c2cInstance.read(alice, "get")
        assert_equal(result.contract_result_data.value, 999)

        result = contractInstance.read(alice, "get")
        assert_equal(result.contract_result_data.value, -1334)

        # Try to spend the funds of a contract
        #
        # First fund the contract with some amount of UTXO,
        # verify that the fund worked (updated state variable)
        # and then try to spend those funds and verify that the
        # spend is rejected by the local PP node because the
        # smart contract has not spent them and thus the outpoint
        # hash is not in the local storage
        #
        # NOTE: spending the DestCallPP UTXOs doesn't require signatures
        # but instead the outpoint hash of the UTXO. This is queried
        # from the runtime storage as the smart contract has not transferred
        # these funds, the outpoint hash is **not** found from the storage
        # and this TX is rejected as invalid
        value -= 555

        (tx, (ext, blk)) = submit_pp_tx(client, tx, alice, value, [utxo.Output(
            value = 555,
            header = 0,
            destination = utxo.DestFundPP(acc_id)
        )])

		# fetch the FundPP UTXO that was just sent
        utxos = [x for x in client.utxos_for(acc_id[2:]) if list(x[1].json()["destination"])[0] == "FundPP"]
        assert_equal(len(utxos), 2)
        assert_equal(utxos[1][1].json()["value"], 555)

        invalid_tx = utxo.Transaction(
            client,
            inputs = [
                utxo.Input(utxos[1][0]),
            ],
            outputs = [
                utxo.Output(
                    value = 555,
                    header = 0,
                    destination = utxo.DestPubkey(alice.public_key)
                ),
            ]
        )

        # # size of the outpoint (32 bytes, 0x10) + the outpoint itself
        # # the outpoint in the witness field is valid but because the
        # # smart contract has not spent the funds, the TX is rejected
        tx.inputs[0].witness = bytearray.fromhex("10" + str(utxos[1][0])[2:])
        assert_equal(client.submit(alice, invalid_tx), None)

if __name__ == '__main__':
    ExampleTest().main()
