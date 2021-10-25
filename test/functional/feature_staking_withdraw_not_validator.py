#!/usr/bin/env python3
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.
"""An example functional test

Bob tries to withdraw a stake which is not his.
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
        self.num_nodes = 1
        # Use self.extra_args to change command-line arguments for the nodes
        self.extra_args = [[]]

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

        ledger = list(client.get_staking_ledger())
        assert_equal(len(ledger[0][1]['unlocking']),0)

        bob = Keypair.create_from_uri('//Bob')
        print("bob's public key: ", bob.public_key)
        # this is alice's locked utxo
        outpoints = ['0x85cf089ac493b7969df7fc1dfd3ed1b880506d7dcfc0e41c71b3676dc2c48b9d']

        (_, _, events) = client.withdraw_stake(bob,outpoints)

        ledger = list(client.get_staking_ledger())
        assert_equal(len(ledger[0][1]['unlocking']),0)

        locked_utxos = list(client.utxos('LockedUtxos'))
        # there should still be 1 utxo locked, no actual withdrawal happened.
        assert_equal(len(locked_utxos),1)


if __name__ == '__main__':
    ExampleTest().main()