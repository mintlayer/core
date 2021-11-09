#!/usr/bin/env python3
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.
"""An example functional test

Using Alice's utxo,  Charlie and Dave stakes for the first time.
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

        alice = Keypair.create_from_uri('//Alice')

        charlie = Keypair.create_from_uri('//Charlie')
        charlie_stash = Keypair.create_from_uri('//Charlie//stash')

        dave = Keypair.create_from_uri('//Dave')
        dave_stash = Keypair.create_from_uri('//Dave//stash')

        # fetch the genesis utxo from storage
        utxos = list(client.utxos_for(alice))

        # there's only 2 record of staking, which are alice and bob.
        assert_equal( len(list(client.staking_count())), 2 )

        # charlie and dave are funded from alice but directly as stakers.
        tx1 = utxo.Transaction(
            client,
            inputs=[
                utxo.Input(utxos[0][0]),
            ],
            outputs=[
                 utxo.Output(
                    value=40000 * COIN,
                    destination=utxo.DestLockForStaking(charlie_stash.public_key, charlie.public_key,'0x7e0dd8c53a47b22451dc3a73b29d72a2ce1405a4191f3c31ff927fea7b0514182f81ffc984364cc85499595eaefc509a06710c5277dcd22ebd7464917dfd9230'),
                     data=None
                ),
                utxo.Output(
                    value=40001 * COIN,
                    destination=utxo.DestLockForStaking(dave_stash.public_key, dave.public_key,'0x0699553a3c5bfa89e41d94a45ceb9103ae9f87089b4a70de4c2a3eb922e1b9362fe0d8868ae4c9d5a9fba98d29b45d2c2630f4936077999f9334da1cca2e37e9'),
                    data=None
                ),
                utxo.Output(
                    value=39999919999 * COIN,
                    destination=utxo.DestPubkey(charlie.public_key),
                    data=None
                )
            ]
        ).sign(alice, [utxos[0][1]])
        client.submit(alice, tx1)

        # there should already be 4 accounts, adding Charlie and Dave in the list.
        assert_equal(len(list(client.staking_count())), 4)

        # Get Charlie
        charlie_count = list(client.get_staking_count(charlie_stash))[0][1]
        # charlie should have 1 locked utxos
        assert_equal(charlie_count[0], 1)
        # charlie should have a total of 50000 * COINS locked
        assert_equal(charlie_count[1], 40000 * COIN)

        # Get Dave
        dave_count = list(client.get_staking_count(dave_stash))[0][1]
        # dave should have 1 locked utxos
        assert_equal(dave_count[0], 1)
        # dave should have a total of 40001 * COINS locked
        assert_equal(dave_count[1], 40001 * COIN)

        # fetch the locked utxos from storage
        locked_utxos = list(client.utxos('LockedUtxos'))
        # there should already be 4 in the list; 1 from alice, 1 from bob, 1 from charlie, 1 from dave
        assert_equal(len(locked_utxos),4)

        # pallet-staking's ledger should have the same number of stakers
        assert_equal( len(list(client.get_staking_ledger())), 4)

if __name__ == '__main__':
    ExampleTest().main()
