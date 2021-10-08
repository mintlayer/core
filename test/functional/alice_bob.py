#!/bin/python

# Alice sends Bob 50 utxos and then Bob sends 30 utxos to Alice and 20 to himself

from substrateinterface import Keypair
import utxo

client = utxo.Client()

alice = Keypair.create_from_uri('//Alice')
bob = Keypair.create_from_uri('//Bob')

# fetch the genesis utxo from storage
utxos = [h for (h, o) in client.utxos_for(alice)]

tx = utxo.Transaction(
    client,
    inputs = [
        utxo.Input(utxos[0]),
    ],
    outputs = [
        utxo.Output(
            value       = 50,
            header      = 0,
            destination = utxo.DestPubkey(bob.public_key)
        ),
    ]
).sign(alice)
client.submit(alice, tx)

tx = utxo.Transaction(
    client,
    inputs = [
        utxo.Input(tx.outpoint(0)),
    ],
    outputs = [
        utxo.Output(
            value       = 30,
            header      = 0,
            destination = utxo.DestPubkey(alice.public_key)
        ),
        utxo.Output(
            value       = 20,
            header      = 0,
            destination = utxo.DestPubkey(bob.public_key)
        ),
    ]
).sign(bob)
client.submit(bob, tx)
