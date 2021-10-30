#!/bin/python

from substrateinterface import Keypair
import utxo
import contract
import time

def assert_(a, b):
    if a == b:
        print("OK")
    else:
        print("ERR")
        sys.exit(1)

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

# fetch the genesis utxo from storage
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

"""
# invalid bytecode
value -= 1

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value=1,
    header=0,
    destination=utxo.DestCreatePP(
        code=[0x00],
        data=[0xed, 0x4b, 0x9d, 0x1b],
    )
))
assert_(contract.getContractAddresses(substrate, blk), None)
    
# invalid value
(invalid_tx, res) = submit_pp_tx(tx, alice, value, utxo.Output(
    value=0,
    header=0,
    destination=utxo.DestCreatePP(
        code="pooltester.wasm",
        data=[0xed, 0x4b, 0x9d, 0x1b],
    )
))
assert_(res, None)
"""

# valid data
value -= 1

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value=1,
    header=0,
    destination=utxo.DestCreatePP(
        code="pooltester.wasm",
        data=[0xed, 0x4b, 0x9d, 0x1b],
    )
))

(ss58, acc_id) = contract.getContractAddresses(substrate, blk)
contractInstance = contract.ContractInstance(ss58, "pooltester.json", substrate)

"""
# verify the initial state of the smart contract
result = contractInstance.read(alice, "get")
assert_(result.contract_result_data.value, 1337)

# valid contract call
value -= 1
msg_data = contractInstance.generate_message_data("flip", {})

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value=1,
    header=0,
    destination=utxo.DestCallPP(
        dest_account=acc_id,
        fund=False,
        input_data=bytes.fromhex(msg_data.to_hex()[2:]),
    )
))
result = contractInstance.read(alice, "get")
assert_(result.contract_result_data.value, -1337)

# call to non-existent smart contract
# try to call smart contract whose destination is Alice's public key
# (i.e., doesn't exist). The call obviously fails and we can verify
# that by checking that the `flip` function did not flip the value
value -= 1
msg_data = contractInstance.generate_message_data("flip", {})

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value=1,
    header=0,
    destination=utxo.DestCallPP(
        dest_account=alice.public_key,
        fund=False,
        input_data=bytes.fromhex(msg_data.to_hex()[2:]),
    )
))
result = contractInstance.read(alice, "get")
assert_(result.contract_result_data.value, -1337)

# Invalid value given 
msg_data = contractInstance.generate_message_data("flip", {})

(invalid_tx, res) = submit_pp_tx(tx, alice, value, utxo.Output(
    value=0,
    header=0,
    destination=utxo.DestCallPP(
        dest_account=alice.public_key,
        fund=False,
        input_data=bytes.fromhex(msg_data.to_hex()[2:]),
    )
))
assert_(res, None)

# test contract-to-p2k transfer from alice to bob
#
# `send_to_pubkey()` first funds the smart contract from alice's funds
# and when the wasm code is executed, the funds are transferred to bob
msg_data = contractInstance.generate_message_data("send_to_pubkey", { "dest": bob.public_key })
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

# verify that bob actually received the utxo
bobs_utxos = [x for x in client.utxos_for(bob)]
assert_(len(bobs_utxos), 1)
assert_(bobs_utxos[0][1].json()['value'], 555)

# test contract-to-p2pk again but this time don't fund the contract
# meaning that after the TX, bob only has the UTXO he received in the previous test case
# and the contract has a UTXO with value 666
msg_data = contractInstance.generate_message_data("send_to_pubkey", { "dest": bob.public_key })
value -= 666

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value = 666,
    header = 0,
    destination = utxo.DestCallPP(
        dest_account = acc_id,
        fund = False,
        input_data = bytes.fromhex(msg_data.to_hex()[2:]),
    )
))

# verify that bob still has only one UTXO
utxos = [x for x in client.utxos_for(bob)]
assert_(len(utxos), 1)

# verify that the contract has one utxo with value 666
utxos = [x for x in client.utxos_for(acc_id[2:])]
assert_(len(utxos), 1)
assert_(utxos[0][1].json()["value"], 666)

# try to call a contract that doesn't exist (alice's public key
# doesn't point to a valid smart contract)
#
# TODO: because we don't have gas refunding, the money is still
# spent, i.e., if the UTXO set is queried, you'll find a UTXO
# with value 888 meaning user just lost his money which is
# not the correct behavior but the implementation is still under way
msg_data = contractInstance.generate_message_data("fund", {})
value -= 888

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value = 888,
    header = 0,
    destination = utxo.DestCallPP(
        dest_account = alice.public_key,
        fund = True,
        input_data = bytes.fromhex(msg_data.to_hex()[2:]),
    )
))

result = contractInstance.read(alice, "get")
print(result.contract_result_data.value)
assert_(result.contract_result_data.value, -1337)

# Try to spend the funds of a contract
#
# First fund the contract with some amount of UTXO,
# verify that the fund worked (updated state variable)
# and then try to spend those funds and verify that the
# spend is rejected by the local PP node because the
# smart contract has not spent them and thus the outpoint
# hash is not in the local storage
#
# NOTE: spending the DestCallPP UTXOs doesn't require signatures
# but instead the outpoint hash of the UTXO. This is queried
# from the runtime storage as the smart contract has not transferred
# these funds, the outpoint hash is **not** found from the storage
# and this TX is rejected as invalid
utxos = [x for x in client.utxos_for(acc_id[2:])]
invalid_tx = utxo.Transaction(
    client,
    inputs=[
        utxo.Input(utxos[0][0]),
    ],
    outputs=[
        utxo.Output(
            value=666,
            header=0,
            destination=utxo.DestPubkey(alice.public_key)
        ),
    ]
)

# size of the outpoint (32 bytes, 0x10) + the outpoint itself
# the outpoint in the witness field is valid but because the
# smart contract has not spent the funds, the TX is rejected
tx.inputs[0].witness = bytearray.fromhex("10" + str(utxos[0][0])[2:])
assert_(client.submit(alice, invalid_tx), None)
"""
msg_data = contractInstance.generate_message_data("fund", {})
value -= 888

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value = 888,
    header = 0,
    destination = utxo.DestCallPP(
        dest_account = alice.public_key,
        fund = True,
        input_data = bytes.fromhex(msg_data.to_hex()[2:]),
    )
))

result = contractInstance.read(alice, "get")
print(result.contract_result_data.value)

value -= 111

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value = 111,
    header = 0,
    destination = utxo.DestCreatePP(
        code = "c2c_tester.wasm",
        data = [0xed, 0x4b, 0x9d, 0x1b],
    )
))

(ss58_c2c, acc_id_c2c) = contract.getContractAddresses(substrate, blk)
c2cInstance = contract.ContractInstance(
    ss58_c2c,
    "c2c_tester.json",
    substrate
)

# verify the initial state of the smart contract
result = c2cInstance.read(alice, "get")
print("value",result.contract_result_data.value)

msg_data = contractInstance.generate_message_data("call_contract", {
	"dest": acc_id_c2c,
	"selector": "0xc6298215",
	"value": 999,
})
value -= 222

(tx, (ext, blk)) = submit_pp_tx(tx, alice, value, utxo.Output(
    value = 222,
    header = 0,
    destination = utxo.DestCallPP(
        dest_account = acc_id,
        fund = True,
        input_data = bytes.fromhex(msg_data.to_hex()[2:]),
    )
))

# verify that the call succeeded
result = c2cInstance.read(alice, "get")
print("value",result.contract_result_data.value)
