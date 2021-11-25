#!/usr/bin/env python3
# Copyright (c) 2021 RBB S.r.l
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.

"""Functional test for issuance

In this test we will create a MLS-01 tokens
"""
# Imports should be in PEP8 ordering (std library first, then third party
# libraries then local imports).
from collections import defaultdict

from substrateinterface import Keypair
import test_framework.mintlayer.utxo as utxo
import hashlib
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


class TokenTest(MintlayerTestFramework):

    def set_test_params(self):
        self.setup_clean_chain = True
        self.num_nodes = 3
        # Use self.extra_args to change command-line arguments for the nodes
        self.extra_args = [[], [], []]

    def setup_network(self):
        self.setup_nodes()
        connect_nodes(self.nodes[0], self.nodes[1])
        self.sync_all([self.nodes[0:1]])

    def custom_method(self):
        """Do some custom behaviour for this test

        Define it in a method here because you're going to use it repeatedly.
        If you think it's useful in general, consider moving it to the base
        MintlayerTestFramework class so other tests can use it."""

        self.log.info("Running custom_method")

    def run_test(self):
        """Main test logic"""
        client = self.nodes[0].rpc_client

        alice = Keypair.create_from_uri('//Alice')
        bob = Keypair.create_from_uri('//Bob')

        # Find an utxo with enough funds
        utxos = list(client.utxos_for(alice))
        # tx1 = utxo.Transaction(
        #     client,
        #     inputs=[
        #         utxo.Input(utxos[0][0]),
        #     ],
        #     outputs=[
        #         utxo.Output(
        #             value=utxo.OVERFLOW_PART_OF_VALUE,
        #             destination=utxo.DestPubkey(alice.public_key),
        #             data=None
        #         ),
        #         # This output prevent reward overflow
        #         utxo.Output(
        #             value=100,
        #             destination=utxo.DestPubkey(bob.public_key),
        #             data=utxo.DataTokenIssuanceV1("TEST", 1000,  1, "")
        #         )
        #
        #     ]
        # ).sign(alice, [utxos[0][1]])
        # token_id = tx1.token_id()
        # client.submit(alice, tx1)
        #
        # tx2 = utxo.Transaction(
        #     client,
        #     inputs=[
        #         utxo.Input(tx1.outpoint(1)),
        #     ],
        #     outputs=[
        #         utxo.Output(
        #             value=99,
        #             destination=utxo.DestPubkey(alice.public_key),
        #             data=None
        #         ),
        #         # utxo.Output(
        #         #     value=0,
        #         #     destination=utxo.DestPubkey(alice.public_key),
        #         #     data=utxo.DataTokenTransferV1(token_id, 1000)
        #         # ),
        #     ]
        # ).sign(bob, [tx1.outputs[0]])
        # client.submit(bob, tx2)
        tx1 = utxo.Transaction(
            client,
            inputs=[
                utxo.Input(utxos[0][0]),
            ],
            outputs=[
                utxo.Output(
                    value=utxo.OVERFLOW_PART_OF_VALUE,
                    destination=utxo.DestPubkey(bob.public_key),
                    data=utxo.DataTokenIssuanceV1("TEST", 1000,  1, "")
                )
            ]
        ).sign(alice, [utxos[0][1]])
        token_id = tx1.token_id()
        client.submit(alice, tx1)

        tx2 = utxo.Transaction(
            client,
            inputs=[
                utxo.Input(tx1.outpoint(0)),
            ],
            outputs=[
                utxo.Output(
                    value=0,
                    destination=utxo.DestPubkey(alice.public_key),
                    data=utxo.DataTokenTransferV1(token_id, 1000)
                ),
                utxo.Output(
                    value=1,
                    destination=utxo.DestPubkey(bob.public_key),
                    data=None
                ),
            ]
        ).sign(bob, tx1.outputs)
        client.submit(bob, tx2)

if __name__ == '__main__':
    TokenTest().main()
