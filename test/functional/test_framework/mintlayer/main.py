#!/bin/python

from substrateinterface import Keypair
import utxo
import contract
import time
import os
import sys

def submit_pp_tx(input_utxo, alice, value, output):
    tx = utxo.Transaction(
        client,
        inputs=[
            utxo.Input(input_utxo.outpoint(0)),
        ],
        outputs=[
            utxo.Output(
                value=value,
                header=0,
                destination=utxo.DestPubkey(alice.public_key)
            ),
            output
        ]
    ).sign(alice, [input_utxo.outputs[0]], [0])
    return tx, client.submit(alice, tx)

client = utxo.Client()
alice = Keypair.create_from_uri('//Alice')
bob = Keypair.create_from_uri('//Bob')
substrate = client.substrate

initial_utxo = [x for x in client.utxos_for(alice) if x[1].value >= 50][0]
value = 10000000000

tx = utxo.Transaction(
    client,
    inputs=[
        utxo.Input(initial_utxo[0]),
    ],
    outputs=[
        utxo.Output(
            value=value,
            header=0,
            destination=utxo.DestPubkey(alice.public_key)
        ),
    ]
).sign(alice, [initial_utxo[1]])
(ext, blk) = client.submit(alice, tx)

# valid data
value -= 111

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value=111,
    header=0,
    destination=utxo.DestCreatePP(
        code=os.path.join(os.path.dirname(__file__), "pooltester.wasm"),
        data=[0xed, 0x4b, 0x9d, 0x1b],
    )
))

(ss58, acc_id) = contract.getContractAddresses(substrate, blk)
contractInstance = contract.ContractInstance(
    ss58,
    os.path.join(os.path.dirname(__file__), "pooltester.json"),
    substrate
)

result = contractInstance.read(alice, "get")
print(result)

msg_data = contractInstance.generate_message_data("send_to_pubkey", { "dest": alice.public_key })
value -= 555

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value = 555,
    header = 0,
    destination = utxo.DestCallPP(
        dest_account = acc_id,
        fund = True,
        input_data = bytes.fromhex(msg_data.to_hex()[2:]),
    )
))

# exts = client.extrinsics(blk)
# print("# of exsts:",len(exts))
# for ext in exts:
#     print(ext)
#     print("")
# blk = substrate.get_block()
# for ext in blk['extrinsics']:
#     print(ext)
# for x in blk:
#     print(x)
# exts = client.extrinsics(blk)
# for ext in exts:
#     print(ext)
# verify that bob actually received the utxo
# bobs_utxos = [x for x in client.utxos_for(bob)]
# print("# of UTXOs bob has:", len(bobs_utxos))
# print("value of bob's UTXO:", bobs_utxos[0][1].json()['value'])
