#!/usr/bin/env python3
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
        # self.sync_all([self.nodes[0:1]])

    def run_test(self):
        """Main test logic"""
        client = self.nodes[0].rpc_client

        substrate = client.substrate

        alice = Keypair.create_from_uri('//Alice')

        # Find a suitable UTXO
        initial_utxo = [x for x in client.utxos_for(alice) if x[1].value >= 50][0]

        tx0 = utxo.Transaction(
            client,
            inputs=[
                utxo.Input(initial_utxo[0]),
            ],
            outputs=[
                utxo.Output(
                    value=50,
                    header=0,
                    destination=utxo.DestPubkey(alice.public_key)
                ),
                utxo.Output(
                    value=0,
                    header=0,
                    destination=utxo.DestCreatePP(
                        code=os.path.join(os.path.dirname(__file__), "code.wasm"),
                        data=[0xed, 0x4b, 0x9d, 0x1b],  # default() constructor selector
                    )
                ),
            ]
        ).sign(alice, [initial_utxo[1]])

        # submit transaction and get the extrinsic and block hashes
        (ext, blk) = client.submit(alice, tx0)

        # each new smart contract instantiation creates a new account
        # fetch this SS58-formatted account address and return it
        # and the hex-encoded account id
        (ss58, acc_id) = contract.getContractAddresses(substrate, blk)

        # create new contract instance which can be used to interact
        # with the instantiated contract
        contractInstance = contract.ContractInstance(
            ss58,
            os.path.join(os.path.dirname(__file__), "metadata.json"),
            substrate
        )

        # read the value of the flipper contract
        result = contractInstance.read(alice, "get")
        print('Current value of "get":', result.contract_result_data)

        msg_data = contractInstance.generate_message_data("flip", {})
        print(ss58, acc_id, msg_data)

        tx1 = utxo.Transaction(
            client,
            inputs=[
                utxo.Input(tx0.outpoint(0)),
            ],
            outputs=[
                utxo.Output(
                    value=49,
                    header=0,
                    destination=utxo.DestPubkey(alice.public_key)
                ),
                utxo.Output(
                    value=0,
                    header=0,
                    destination=utxo.DestCallPP(
                        dest_account=acc_id,
                        input_data=bytes.fromhex(msg_data.to_hex()[2:]),
                    )
                ),
            ]
        ).sign(alice, [tx0.outputs[0]], [0])
        (ext_hash, blk_hash) = client.submit(alice, tx1)

        result = contractInstance.read(alice, "get")
        print('Current value of "get":', result.contract_result_data)


if __name__ == '__main__':
    ExampleTest().main()
