#!/bin/python

# Alice sends herself 50 and 100 utxos and then sends another 60 to herself
# using the previous transaction

from substrateinterface import Keypair
import test_framework.mintlayer.utxo as utxo

client = utxo.Client()

alice = Keypair.create_from_uri('//Alice')

# Find an utxo with enough funds
utxos = [h for (h, o) in client.utxos_for(alice) if o.value >= 150]

tx1 = utxo.Transaction(
    client,
    inputs = [
        utxo.Input(utxos[0]),
    ],
    outputs = [
        utxo.Output(
            value       = 50,
            header      = 0,
            destination = utxo.DestPubkey(alice.public_key)
        ),
        utxo.Output(
            value       = 100,
            header      = 0,
            destination = utxo.DestPubkey(alice.public_key)
        ),

    ]
).sign(alice)
client.submit(alice, tx1)

tx2 = utxo.Transaction(
    client,
    inputs = [
        # spend the 100 utxo output (index 1)
        utxo.Input(tx1.outpoint(1)),
    ],
    outputs = [
        utxo.Output(
            value       = 60,
            header      = 0,
            destination = utxo.DestPubkey(alice.public_key)
        ),
    ]
).sign(alice)
client.submit(alice, tx2)
