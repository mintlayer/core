#!/usr/bin/env python3
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.
"""An example functional test

Bob stakes an extra 40_000 utxo... but he inputted the wrong controller account
"""

from substrateinterface import Keypair
import test_framework.mintlayer.utxo as utxo

from test_framework.test_framework import MintlayerTestFramework
from test_framework.util import (
    assert_equal,
    connect_nodes,
    wait_until,
)
from test_framework.messages import COIN


class ExampleTest(MintlayerTestFramework):
    # Each functional test is a subclass of the MintlayerTestFramework class.

    # Override the set_test_params(), add_options(), setup_chain(), setup_network()
    # and setup_nodes() methods to customize the test setup as required.

    def set_test_params(self):
        """Override test parameters for your individual test.

        This method must be overridden and num_nodes must be exlicitly set."""
        self.setup_clean_chain = True
        self.num_nodes = 2
        # Use self.extra_args to change command-line arguments for the nodes
        self.extra_args = [['--alice'],['--bob']]

        # self.log.info("I've finished set_test_params")  # Oops! Can't run self.log before run_test()

    def setup_network(self):
        """Setup the test network topology

        Often you won't need to override this, since the standard network topology
        (linear: node0 <-> node1 <-> node2 <-> ...) is fine for most tests.

        If you do override this method, remember to start the nodes, assign
        them to self.nodes, connect them and then sync."""

        self.setup_nodes()

    def custom_method(self):
        """Do some custom behaviour for this test

        Define it in a method here because you're going to use it repeatedly.
        If you think it's useful in general, consider moving it to the base
        MintlayerTestFramework class so other tests can use it."""

        self.log.info("Running custom_method")

    def run_test(self):
        client = self.nodes[0].rpc_client

        bob = Keypair.create_from_uri('//Bob')
        alice_stash = Keypair.create_from_uri('//Alice//stash')

        # fetch the genesis utxo from storage
        utxos = list(client.utxos_for(alice_stash))

        # Get Bob stash
        orig_count = list(client.get_staking_count(alice_stash))[0][1]
        # there should be 1 count of bob's locked utxo
        assert_equal(orig_count[0],1)
        # the amount that bob locked is 40_000 * MLT_UNIT
        assert_equal(orig_count[1],40000 * COIN)

        tx1 = utxo.Transaction(
            client,
            inputs=[
                utxo.Input(utxos[0][0]),
            ],
            outputs=[
                utxo.Output(
                    value=40000 * COIN,
                    header=0,
                    destination=utxo.DestLockExtraForStaking(alice_stash.public_key, bob.public_key)
                ),
            ]
        ).sign(alice_stash, [utxos[0][1]])

        client.submit(alice_stash, tx1)

        # Get Bob's stash
        new_count = list(client.get_staking_count(alice_stash))[0][1]

        # there should still be only 1.
        assert_equal(new_count[0],1)

        # the stake stays the same
        assert_equal(new_count[1],40000 * COIN)

if __name__ == '__main__':
    ExampleTest().main()
