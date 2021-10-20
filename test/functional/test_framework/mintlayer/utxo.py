# Copyright (c) 2021 RBB S.r.l

import substrateinterface
from substrateinterface import SubstrateInterface, Keypair
from substrateinterface.exceptions import SubstrateRequestException
from substrateinterface.utils.ss58 import ss58_decode
import scalecodec
import os
import logging

""" Client. A thin wrapper over SubstrateInterface """
class Client:
    def __init__(self, url="ws://127.0.0.1", port=9944):
        source_dir = os.path.dirname(os.path.abspath(__file__))
        types_file = os.path.join(source_dir, "..", "..", "custom-types.json")
        custom_type_registry = scalecodec.type_registry.load_type_registry_file(types_file)
        self.log = logging.getLogger('TestFramework.client')

        self.substrate = SubstrateInterface(
            url=url + ":" + str(port),
            ss58_format=42,
            type_registry_preset='substrate-node-template',
            type_registry=custom_type_registry
        )

    """ SCALE-encode given object in JSON format """
    def encode_obj(self, ty, obj):
        return self.substrate.encode_scale(ty, obj)

    """ SCALE-encode given object """
    def encode(self, obj):
        return self.encode_obj(obj.type_string(), obj.json())

    """ Hash of a SCALE-encoded version of given JSON object """
    def hash_of(self, ty, obj):
        encoded = self.encode_obj(ty, obj).data
        return '0x' + str(substrateinterface.utils.hasher.blake2_256(encoded))

    """ Query the node for the list of utxos """
    def utxos(self):
        query = self.substrate.query_map(
            module="Utxo",
            storage_function="UtxoStore",
            ignore_decoding_errors=False
        )

        return ((h, Output.load(o.value)) for (h, o) in query)

    """ Get UTXOs for given key """
    def utxos_for(self, keypair):
        if type(keypair) == str:
            matching = lambda e: e[1].destination.get_pubkey() == keypair
        else:
            matching = lambda e: e[1].destination.get_pubkey() == keypair.public_key
        return filter(matching, self.utxos())

    """ Submit a transaction onto the blockchain """
    def submit(self, keypair, tx):
        call = self.substrate.compose_call(
            call_module = 'Utxo',
            call_function = 'spend',
            call_params = { 'tx': tx.json() },
        )
        extrinsic = self.substrate.create_signed_extrinsic(call=call, keypair=keypair)
        self.log.debug("extrinsic submitted...")

        try:
            receipt = self.substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
            self.log.debug("Extrinsic '{}' sent and included in block '{}'".format(receipt.extrinsic_hash, receipt.block_hash))
            return (receipt.extrinsic_hash, receipt.block_hash)
        except SubstrateRequestException as e:
            self.log.info("Failed to send: {}".format(e))

class Destination():
    @staticmethod
    def load(obj):
        if 'Pubkey' in obj:
            return DestPubkey.load(obj['Pubkey'])
        if 'CreatePP' in obj:
            return DestCreatePP.load(obj['CreatePP'])
        if 'CallPP' in obj:
            return DestCallPP.load(obj['CallPP'])
        return None

    def type_string(self):
        return 'Destination'

    def get_pubkey(self):
        return None

# Only Schnorr pubkey type supported now.
class DestPubkey(Destination):
    def __init__(self, pubkey):
        self.pubkey = pubkey

    @staticmethod
    def load(obj):
        return DestPubkey(obj)

    def json(self):
        return { 'Pubkey': self.pubkey }

    def get_pubkey(self):
        return self.pubkey

class DestCreatePP(Destination):
    def __init__(self, code, data):
        if type(code) == str:
            with open(code, "rb") as file:
                code = file.read()
        self.code = code
        self.data = data

    @staticmethod
    def load(obj):
        # the type of obj['code'] is str but instead
        # containing a file path, it contains the bytecode
        # of the smart contract.
        # Because it's str the constructor tries to use it a file path
        # and thus incorrectly constructs the DestCreatePP object
        #
        # convert the bytecode str representation to a byte vector
        code = bytes.fromhex(obj['code'][2:])
        return DestCreatePP(code, obj['data'])

    def json(self):
        return { 'CreatePP': { 'code': self.code, 'data': self.data } }

class DestCallPP(Destination):
    def __init__(self, dest_account, fund, input_data):
        self.acct = dest_account
        self.fund = fund
        self.data = input_data

    @staticmethod
    def load(obj):
        return DestCallPP(ss58_decode(obj['dest_account']), obj['fund'], obj['input_data'])

    def json(self):
        return { 'CallPP': { 'dest_account': self.acct, 'fund': self.fund, 'input_data': self.data } }

    def get_pubkey(self):
        return str(self.acct)

class Output():
    def __init__(self, value, header, destination):
        self.value = value
        self.header = header
        self.destination = destination

    @staticmethod
    def load(obj):
        dest = Destination.load(obj['destination'])
        return Output(obj['value'], obj['header'], dest)

    def type_string(self):
        return 'TransactionOutput'

    def json(self):
        return {
            'value': self.value,
            'header': self.header,
            'destination': self.destination.json(),
        }


class Input():
    def __init__(self, outpoint, lock = '0x', witness = '0x'):
        self.outpoint = outpoint
        self.lock = lock
        self.witness = witness

    def type_string(self):
        return 'TransactionInput'

    def json(self):
        return {
            'outpoint': str(self.outpoint),
            'lock': self.lock,
            'witness': self.witness,
        }

class Transaction():
    def __init__(self, client, inputs, outputs, time_lock = 0):
        self.client = client
        self.inputs = inputs
        self.outputs = outputs
        self.time_lock = time_lock

    def type_string(self):
        return 'Transaction'

    def json(self):
        return {
            'inputs': [ i.json() for i in self.inputs ],
            'outputs': [ o.json() for o in self.outputs ],
            'time_lock': self.time_lock
        }

    """ Get data to be signed for this transaction """
    def signature_data(self, spent_utxos, idx):
        # Create the signature message. Only the default sighash supported for now.
        utxos_hash = self.client.hash_of('Vec<TransactionOutput>',
                [ u.json() for u in spent_utxos ])
        outpoints_hash = self.client.hash_of('Vec<H256>',
                [ str(i.outpoint) for i in self.inputs ])
        outputs_hash = self.client.hash_of('Vec<TransactionOutput>',
                [ o.json() for o in self.outputs ])

        sigdata = {
            'sighash': 0,
            'inputs': { 'SpecifiedPay': (outpoints_hash, utxos_hash, idx) },
            'outputs': { 'All': outputs_hash },
            'time_lock': self.time_lock,
            'codesep_pos': 0xffffffff
        }
        return self.client.encode_obj('SignatureData', sigdata)

    """ Sigh the transaction inputs listed in input_idxs (all if missing) """
    def sign(self, keypair, spent_utxos, input_idxs = None):
        assert len(self.inputs) == len(spent_utxos), "1 utxo per input required"
        input_idxs = input_idxs or range(len(self.inputs))
        for idx in input_idxs:
            signature = keypair.sign(self.signature_data(spent_utxos, idx))
            self.inputs[idx].witness = signature
        return self

    """ Get UTXO ID of n-th output of this transaction """
    def outpoint(self, n):
        outpt = {
            'transaction': self.json(),
            'index': n
        }
        encoded = self.client.substrate.encode_scale('Outpoint', outpt)
        return '0x' + str(substrateinterface.utils.hasher.blake2_256(encoded.data))
