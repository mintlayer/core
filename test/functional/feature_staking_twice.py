#!/usr/bin/env python3
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.
"""An example functional test

Send a transaction from Alice to Charlie, then Charlie stakes for the first time.. and an extra one.
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
                    value=70000 * COIN,
                    header=0,
                    destination=utxo.DestPubkey(charlie.public_key)
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
                    header=0,
                    destination=utxo.DestLockForStaking(charlie_stash.public_key, charlie.public_key,'0xa03bcfaac6ebdc26bb9c256c51b08f9c1c6d4569f48710a42939168d1d7e5b6086b20e145e97158f6a0b5bff2994439d3320543c8ff382d1ab3e5eafffaf1a18')
                ),
                 utxo.Output(
                    value=10000 * COIN,
                    header=0,
                    destination=utxo.DestLockExtraForStaking(charlie.public_key)
                ),
                utxo.Output(
                    value=19998 * COIN,
                    header=0,
                    destination=utxo.DestPubkey(charlie.public_key)
                ),
            ]
        ).sign(charlie, tx1.outputs)


        client.submit(charlie, tx2)

        updated_count = list(client.staking_count())

        # there should already be 3 accounts, adding Charlie in the list.
        assert_equal(len(updated_count), 3)

        # Get Charlie
        charlie_count = list(filter(lambda e: e[0].value == charlie.public_key , updated_count))[0][1]
        print("CHARLIE COUNT: ", charlie_count)
        # charlie should have 2 locked utxos
        assert_equal(charlie_count[0], 2)
#
#         # charlie should have a total of 50000 * COINS locked
#         assert_equal(charlie_count[1], 50000 * COIN)
#
#         # fetch the locked utxos from storage
#         locked_utxos = list(client.utxos('LockedUtxos'))
#         # there should already be 4 in the list; 1 from alice, 1 from bob, 2 from charlie
#         assert_equal(len(locked_utxos),4)


if __name__ == '__main__':
    ExampleTest().main()
