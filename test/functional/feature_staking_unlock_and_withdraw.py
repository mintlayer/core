#!/usr/bin/env python3
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.
"""An example functional test

Alice wants to unlock her locked utxos, and withdraw; leaving Bob all alone.
This was tested with Block time of 20 seconds.

"""

from substrateinterface import Keypair
import test_framework.mintlayer.utxo as utxo
import time
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
        connect_nodes(self.nodes[1], self.nodes[0])

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

        alice_stash = Keypair.create_from_uri('//Alice//stash')
        alice = Keypair.create_from_uri('//Alice')

        # fetch the genesis utxo from storage
        utxos = list(client.utxos_for(alice_stash))

        # Alice's locked utxo
        locked_utxos = list(map(lambda e: e[0].value, list(client.locked_utxos_for(alice_stash))))

        # there's 2 records of staking, Alice's and Bob's.
        assert_equal( len(list(client.staking_count())), 2 )

        (_, _, events) = client.unlock_request_for_withdrawal(alice_stash)

        assert_equal(events[0].value['module_id'],'Staking')
        assert_equal(events[0].value['event_id'], 'Chilled')

        assert_equal(events[1].value['module_id'],'Staking')
        assert_equal(events[1].value['event_id'], 'Unbonded')

        assert_equal(events[2].value['module_id'],'Utxo')
        assert_equal(events[2].value['event_id'], 'StakeUnlocked')

        ledger = list(client.get_staking_ledger())

        assert_equal(len(ledger),2)

        while client.current_era() < client.withdrawal_era(alice):
            time.sleep(1)

        (_, _, w_events) = client.withdraw_stake(alice_stash)


        assert_equal(w_events[0].value['module_id'],'Staking')
        assert_equal(w_events[0].value['event_id'], 'Withdrawn')

        assert_equal(w_events[1].value['module_id'],'Utxo')
        assert_equal(w_events[1].value['event_id'], 'StakeWithdrawn')

        assert_equal( len(list(client.staking_count())), 1)

        updated_ledger = list(client.get_staking_ledger())
        assert_equal(len(updated_ledger),1)


if __name__ == '__main__':
    ExampleTest().main()
