#!/bin/python

# Copyright (c) 2021 RBB S.r.l

import substrateinterface
from substrateinterface import SubstrateInterface, Keypair
from substrateinterface.exceptions import SubstrateRequestException
import scalecodec
import json

# fetch the contract account id from the block using its hash

# TODO limitation here is that only one smart contract can be
# created per block in order for this to work.
# think of something a little more elegant
def getContractAddresses(substrate, blk_hash):
    events = substrate.get_events(blk_hash)
    for event in events:
        if event.event_module.name == "System" and event.event.name == "NewAccount":
            return (
                event.params[0]['value'],
                "0x" + substrateinterface.utils.ss58.ss58_decode(event.params[0]['value'])
            )


class ContractInstance():
    def __init__(self, ss58, metadata, substrate):
        self.metadata = metadata
        self.substrate = substrate
        self.interface = substrateinterface.contracts.ContractInstance.create_from_address(
            contract_address = ss58,
            metadata_file = metadata,
            substrate = substrate
        )

    def read(self, keypair, method):
        return self.interface.read(keypair, method)

    def generate_message_data(self, method, params):
        metadata = json.load(open(self.metadata))
        ctr_tmp = substrateinterface.contracts.ContractMetadata(metadata, self.substrate)
        return ctr_tmp.generate_message_data(method, params)
