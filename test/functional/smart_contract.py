#!/bin/python

# This example code demonstrates how to instantiate the flipper ink!
# smart contract, query its state using get(), change the state by
# issuing a utxo that contains the OP_CALL opcode and then querying
# the state of the variable again

from substrateinterface import Keypair
import utxo
import contract
import sys

client = utxo.Client()
substrate = client.substrate

alice = Keypair.create_from_uri('//Alice')

# Find a suitable UTXO
initial_utxo = [h for (h, u) in client.utxos_for(alice) if u.value >= 50][0]

tx = utxo.Transaction(
    client,
    inputs = [
        utxo.Input(initial_utxo),
    ],
    outputs = [
        utxo.Output(
            value       = 50,
            header      = 0,
            destination = utxo.DestPubkey(alice.public_key)
        ),
        utxo.Output(
            value       = 0,
            header      = 0,
            destination = utxo.DestCreatePP(
                code = "code.wasm",
                data = [0xed, 0x4b, 0x9d, 0x1b], # default() constructor selector
            )
        ),
    ]
).sign(alice)

# submit transaction and get the extrinsic and block hashes
(ext, blk) = client.submit(alice, tx)

# each new smart contract instantiation creates a new account
# fetch this SS58-formatted account address and return it
# and the hex-encoded account id
(ss58, acc_id) = contract.getContractAddresses(substrate, blk)

# create new contract instance which can be used to interact
# with the instantiated contract
contractInstance = contract.ContractInstance(
    ss58,
    "metadata.json",
    substrate
)

# read the value of the flipper contract
result = contractInstance.read(alice, "get")
print('Current value of "get":', result.contract_result_data)

msg_data = contractInstance.generate_message_data("flip", {})
print(ss58, acc_id, msg_data)

tx = utxo.Transaction(
    client,
    inputs = [
        utxo.Input(tx.outpoint(0)),
    ],
    outputs = [
        utxo.Output(
            value       = 49,
            header      = 0,
            destination = utxo.DestPubkey(alice.public_key)
        ),
        utxo.Output(
            value       = 0,
            header      = 0,
            destination = utxo.DestCallPP(
                dest_account = acc_id,
                input_data = bytes.fromhex(msg_data.to_hex()[2:]),
            )
        ),
    ]
).sign(alice)
(ext_hash, blk_hash) = client.submit(alice, tx)

result = contractInstance.read(alice, "get")
print('Current value of "get":', result.contract_result_data)
