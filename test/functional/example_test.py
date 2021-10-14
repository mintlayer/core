#!/usr/bin/env python3
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.
"""An example functional test

The module-level docstring should include a high-level description of
what the test is doing. It's the first thing people see when they open
the file and should give the reader information about *what* the test
is testing and *how* it's being tested
"""
# Imports should be in PEP8 ordering (std library first, then third party
# libraries then local imports).
from collections import defaultdict

from substrateinterface import Keypair
import test_framework.mintlayer.utxo as utxo

# Avoid wildcard * imports if possible
# from test_framework.blocktools import (create_block, create_coinbase)
# from test_framework.mininode import (
#     CInv,
#     P2PInterface,
#     mininode_lock,
#     msg_block,
#     msg_getdata,
#     network_thread_join,
#     network_thread_start,
# )
from test_framework.test_framework import MintlayerTestFramework
from test_framework.util import (
    assert_equal,
    connect_nodes,
    wait_until,
)


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

    # Use add_options() to add specific command-line options for your test.
    # In practice this is not used very much, since the tests are mostly written
    # to be run in automated environments without command-line options.
    # def add_options()
    #     pass

    # Use setup_chain() to customize the node data directories. In practice
    # this is not used very much since the default behaviour is almost always
    # fine
    # def setup_chain():
    #     pass

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

    # Use setup_nodes() to customize the node start behaviour (for example if
    # you don't want to start all nodes at the start of the test).
    # def setup_nodes():
    #     pass

    def custom_method(self):
        """Do some custom behaviour for this test

        Define it in a method here because you're going to use it repeatedly.
        If you think it's useful in general, consider moving it to the base
        MintlayerTestFramework class so other tests can use it."""

        self.log.info("Running custom_method")

    def run_test(self):
        """Main test logic"""
        alice = Keypair.create_from_uri('//Alice')

        available_utxos = self.nodes[0].rpc_client.utxos_for(alice)

        utxos = [h for (h, o) in available_utxos if o.value >= 150]

        tx1 = utxo.Transaction(
            self.nodes[0].rpc_client,
            inputs=[
                utxo.Input(utxos[0]),
            ],
            outputs=[
                utxo.Output(
                    value=50,
                    header=0,
                    destination=utxo.DestPubkey(alice.public_key)
                ),
                utxo.Output(
                    value=100,
                    header=0,
                    destination=utxo.DestPubkey(alice.public_key)
                ),

            ]
        ).sign(alice)
        extrinsic = self.nodes[0].rpc_client.submit(alice, tx1)
        self.log.info("extrinsic submitted... '{}'".format(extrinsic))

        receipt = self.nodes[0].rpc_client.get_receipt(extrinsic, True)
        self.log.info("Extrinsic '{}' sent and included in block '{}'".format(receipt[0], receipt[1]))
        assert_equal(receipt[0].replace("0x", ""), extrinsic.extrinsic_hash.replace("0x", ""))


if __name__ == '__main__':
    ExampleTest().main()
