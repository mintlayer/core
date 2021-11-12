#!/usr/bin/env python3
# Copyright (c) 2021 RBB S.r.l
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.

""" An example functional test with custom chainspec """
import requests
from substrateinterface import Keypair
import test_framework.mintlayer.utxo as utxo

from test_framework.test_framework import MintlayerTestFramework
from test_framework.util import (
    assert_equal,
    connect_nodes,
    wait_until,
)

def insert_key(port, type, sk, pk):
    payload = '{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "author_insertKey",
        "params": [ "%s", "%s", "%s" ]
    }' % (type, sk, pk)
    headers = {'content-type': 'application/json', 'Accept-Charset': 'application/json'}
    r = requests.post("http://127.0.0.1:" + port, data=payload, headers=headers)

class ExampleTest(MintlayerTestFramework):
    def set_test_params(self):
        self.setup_clean_chain = True
        self.num_nodes = 4
        self.extra_args = [
            ['--chain assets/custom_chain.json --validator --port 9940 --rpc-port 9942 --ws-port 9941'],
            ['--chain assets/custom_chain.json --validator --port 9950 --rpc-port 9952 --ws-port 9951'],
            ['--chain assets/custom_chain.json --validator --port 9960 --rpc-port 9962 --ws-port 9961'],
            ['--chain assets/custom_chain.json --validator --port 9970 --rpc-port 9972 --ws-port 9971']
        ]
 
    def setup_network(self):
        self.setup_nodes()

		# TODO: get public keys?
		# TODO: verify json rpc output is correct
        insert_key(9942, "aura", "//Alice", "")
        insert_key(9942, "gran", "//Alice", "")

        insert_key(9952, "aura", "//Bob", "")
        insert_key(9952, "gran", "//Bob", "")

        insert_key(9962, "aura", "//Charlie", "")
        insert_key(9962, "gran", "//Charlie", "")

        insert_key(9972, "aura", "//Dave", "")
        insert_key(9972, "gran", "//Dave", "")

		# TODO: verity that this doesn't destroy storage
        self.restart_nodes()

    def custom_method(self):
        self.log.info("Running custom_method")

    def run_test(self):
        client = self.nodes[0].rpc_client

        alice = Keypair.create_from_uri('//Alice')
        bob = Keypair.create_from_uri('//Bob')

if __name__ == '__main__':
    ExampleTest().main()
