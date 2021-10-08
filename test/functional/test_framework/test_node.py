#!/usr/bin/env python3
# Copyright (c) 2017 The Bitcoin Core developers
# Distributed under the MIT software license, see the accompanying
# file COPYING or http://www.opensource.org/licenses/mit-license.php.
"""Class for mintlayer node under test"""

import decimal
import errno
import http.client
import json
import logging
import mintlayer
import os
import re
import subprocess
import time

# from .authproxy import JSONRPCException
from .util import (
    assert_equal,
    delete_cookie_file,
    get_rpc_proxy,
    rpc_url,
    wait_until,
    p2p_port,
    rpc_port,
)

# For Python 3.4 compatibility
JSONDecodeError = getattr(json, "JSONDecodeError", ValueError)

MINTLAYER_PROC_WAIT_TIMEOUT = 60

class TestNode():
    """A class for representing a Mintlayer node under test.

    This class contains:

    - state about the node (whether it's running, etc)
    - a Python subprocess.Popen object representing the running process
    - an RPC connection to the node
    - one or more P2P connections to the node


    To make things easier for the test writer, any unrecognised messages will
    be dispatched to the RPC connection."""

    def __init__(self, i, dirname, extra_args, rpchost, timewait, binary, stderr, mocktime, coverage_dir, use_cli=False):
        self.index = i
        self.datadir = os.path.join(dirname, "node" + str(i))
        self.rpchost = rpchost
        if timewait:
            self.rpc_timeout = timewait
        else:
            # Wait for up to 600 seconds for the RPC server to respond
            self.rpc_timeout = 600
        if binary is None:
            self.binary = os.getenv("NODEEXE", "node-template")
        else:
            self.binary = binary
        self.stderr = stderr
        self.coverage_dir = coverage_dir
        # Most callers will just need to add extra args to the standard list below. For those callers that need more flexibity, they can just set the args property directly.
        self.extra_args = extra_args
        port_rpc = rpc_port(i)
        port_p2p = p2p_port(i)
        self.args = [self.binary, "--dev",
                     "--base-path", self.datadir,
                     "--log", "trace",
                     "--name", "testnode%d" % i,
                     "--ws-port", "{}".format(port_rpc),
                     "--port", "{}".format(port_p2p),
                     "--reserved-only"
                     ]

        self.running = False
        self.process = None
        self.rpc_connected = False
        self.rpc_client = None
        self.rpc = None
        self.url = None
        self.log = logging.getLogger('TestFramework.node%d' % i)
        self.cleanup_on_exit = True # Whether to kill the node when this object goes away
        self.peer_id = None
        self.stdout_file = None
        self.stderr_file = None

        self.p2ps = []

    def __del__(self):
        # close logs
        if self.stdout_file is not None:
            self.stdout_file.close()
        if self.stderr_file is not None:
            self.stderr_file.close()
        # Ensure that we don't leave any mintlayer node processes lying around after
        # the test ends
        if self.process and self.cleanup_on_exit:
            # Should only happen on test failure
            # Avoid using logger, as that may have already been shutdown when
            # this destructor is called.
            print("Cleaning up leftover process")
            self.process.kill()

    def __getattr__(self, name):
        """Dispatches any unrecognised messages to the RPC connection."""
        assert self.rpc_connected and self.rpc is not None, "Error: no RPC connection"
        return getattr(self.rpc, name)

    def start(self, extra_args=None, stderr=None, *args, **kwargs):
        """Start the node."""
        if extra_args is None:
            extra_args = self.extra_args
        if stderr is None:
            stderr = self.stderr
        self.stdout_file = open(os.path.join(self.datadir, "stdout.txt"), "w")
        if stderr is None:
            self.stderr_file = open(os.path.join(self.datadir, "stderr.txt"), "w")
            stderr = self.stderr_file
        # Delete any existing cookie file -- if such a file exists (eg due to
        # unclean shutdown), it will get overwritten anyway by mintlayer node, and
        # potentially interfere with our attempt to authenticate
        delete_cookie_file(self.datadir)
        run_command = self.args + extra_args
        self.process = subprocess.Popen(run_command, stderr=stderr, stdout=self.stdout_file, *args, **kwargs)
        self.running = True
        self.log.debug("node started, waiting for RPC to come up")
        self.log.debug("node was started with command: %s" % run_command)

    def wait_for_rpc_connection(self):

        """Sets up an RPC connection to the Mintlayer node process. Returns False if unable to connect."""
        # Poll at a rate of four times per second
        poll_per_s = 4
        for _ in range(poll_per_s * self.rpc_timeout):
            node_index = self.index
            assert self.process.poll() is None, "Mintlayer node exited with status %i during initialization" % self.process.returncode
            try:
                port = rpc_port(node_index)
                url = "ws://127.0.0.1"  # TODO: move this to a parameter in the constructor
                rpc_client = mintlayer.utxo.Client(url, port)
                # let's run some functions that show that the node is running
                rpc_client.utxos()
                rpc_client.substrate.get_block_hash(0)
                node_id_response = rpc_client.substrate.rpc_request("system_localPeerId", [])
                self.peer_id = node_id_response["result"]

                # If the calls succeeds then the RPC connection is up
                self.rpc_client = rpc_client
                self.rpc_connected = True
                self.url = url + ":" + str(port)
                self.log.debug("RPC successfully started")
                return

            except IOError as e:
                if e.errno != errno.ECONNREFUSED:  # Port not yet open?
                    raise  # unknown IO error

            except ValueError as e:  # cookie file not found and no rpcuser or rpcassword. mintlayer node still starting
                if "No RPC credentials" not in str(e):
                    raise
            time.sleep(1.0 / poll_per_s)
        raise AssertionError("Unable to connect to mintlayer node")

    def get_wallet_rpc(self, wallet_name):
        assert self.rpc_connected
        assert self.rpc
        wallet_path = "wallet/%s" % wallet_name
        return self.rpc / wallet_path

    def stop_node(self):
        """Stop the node."""
        if not self.running:
            return
        self.log.debug("Stopping node")
        try:
            self.process.terminate()
        except http.client.CannotSendRequest:
            self.log.exception("Unable to stop node.")
        del self.p2ps[:]

    def is_node_stopped(self):
        """Checks whether the node has stopped.

        Returns True if the node has stopped. False otherwise.
        This method is responsible for freeing resources (self.process)."""
        if not self.running:
            return True
        return_code = self.process.poll()
        if return_code is None:
            return False

        # process has stopped. Assert that it didn't return an error code.
        assert_equal(return_code, 0)
        self.running = False
        self.process = None
        self.rpc_connected = False
        self.rpc = None
        self.log.debug("Node stopped")
        return True

    def wait_until_stopped(self, timeout=MINTLAYER_PROC_WAIT_TIMEOUT):
        wait_until(self.is_node_stopped, timeout=timeout)

    def node_encrypt_wallet(self, passphrase):
        """"Encrypts the wallet.

        This causes node to shutdown, so this method takes
        care of cleaning up resources."""
        self.encryptwallet(passphrase)
        self.wait_until_stopped()

    def add_p2p_connection(self, p2p_conn, *args, **kwargs):
        """Add a p2p connection to the node.

        This method adds the p2p connection to the self.p2ps list and also
        returns the connection to the caller."""
        if 'dstport' not in kwargs:
            kwargs['dstport'] = p2p_port(self.index)
        if 'dstaddr' not in kwargs:
            kwargs['dstaddr'] = '127.0.0.1'

        p2p_conn.peer_connect(*args, **kwargs)
        self.p2ps.append(p2p_conn)

        return p2p_conn

    @property
    def p2p(self):
        """Return the first p2p connection

        Convenience property - most tests only use a single p2p connection to each
        node, so this saves having to write node.p2ps[0] many times."""
        assert self.p2ps, "No p2p connection"
        return self.p2ps[0]

    def disconnect_p2ps(self):
        """Close all p2p connections to the node."""
        for p in self.p2ps:
            p.peer_disconnect()
        del self.p2ps[:]
