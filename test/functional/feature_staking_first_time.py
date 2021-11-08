#!/usr/bin/env python3
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.
"""An example functional test

Send a transaction from Alice to Charlie, then Charlie stakes for the first time.
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

        alice = Keypair.create_from_uri('//Alice')
        charlie = Keypair.create_from_uri('//Charlie')
        charlie_stash = Keypair.create_from_uri('//Charlie//stash')

        # fetch the genesis utxo from storage
        utxos = list(client.utxos_for(alice))

        # there's only 2 record of staking, which are alice and bob.
        assert_equal( len(list(client.staking_count())), 2 )

        tx1 = utxo.Transaction(
            client,
            inputs=[
                utxo.Input(utxos[0][0]),
            ],
            outputs=[
                utxo.Output(
                    value=50000 * COIN,
                    destination=utxo.DestPubkey(charlie_stash.public_key),
                    data=None
                ),
            ]
        ).sign(alice, [utxos[0][1]])
        client.submit(alice, tx1)

        tx2 = utxo.Transaction(
            client,
            inputs=[
                utxo.Input(tx1.outpoint(0)),
            ],
            outputs=[
                utxo.Output(
                    value=40000 * COIN,
                    destination=utxo.DestLockForStaking(charlie_stash.public_key, charlie.public_key,'0xa03bcfaac6ebdc26bb9c256c51b08f9c1c6d4569f48710a42939168d1d7e5b6086b20e145e97158f6a0b5bff2994439d3320543c8ff382d1ab3e5eafffaf1a18'),
                    data=None
                ),
                utxo.Output(
                    value=9999 * COIN,
                    destination=utxo.DestPubkey(charlie_stash.public_key),
                    data=None
                ),
            ]
        ).sign(charlie_stash, tx1.outputs)
        (_,_,events) = client.submit(charlie_stash, tx2)

        # there should already be 3 staking, adding Charlie in the list.
        assert_equal( len(list(client.staking_count())), 3 )

        # pallet-staking's ledger should have the same number of stakers
        assert_equal( len(list(client.get_staking_ledger())), 3)

if __name__ == '__main__':
    ExampleTest().main()
