# Transactions in Mintlayer

## UTXO overview

**TODO maybe system/model instead of structure?**
Mintlayer uses the Bitcoin UTXO structure instead of the account-based models of Ethereum, Ripple, Stellar, and others.
**TODO need explanation of this sentence, it is not clear to me**
Since each transaction's output is stored separately (even when sent to a single address), it is only possible to spend the entire transaction’s output.

There are three essential reasons for choosing the UTXO model: 

- It is compatible with technologies already implemented in Bitcoin, such atomic swaps and the Lightning Network.
 
**TODO - why does this improve privacy?**
- It is more privacy-oriented: a single wallet usually utilizes multiple addresses, making it difficult and sometimes impossible to determine which addresses belong to whichs user. 
 
- Payments can be batched together (aggregated) in a single transaction, saving a considerable amount of the space otherwise required for making a single transaction per payment.  

## How to send a transaction in Mintlayer node
There are three destination types for transaction outputs : 
- Pubkey (Currently, only Schnorr public keys are supported)
- LockForStaking
- LockExtraForStaking

A general Mintlayer transaction looks something like this: 

**TODO Not sure we want this in Rust code. Too developer specific. Not clear what H256 is, witness, lock**
**TODO possibly add a link to information about the utxo system**
**TODO if we go for the rust struct, then we need the data field. Also, what is this field?**
**TODO timelock is not a string..."**
```rust
Transaction {
    inputs: [
        TransactionInput {
	    outpoint: <H256>,
            witness: <signature>, 
            lock: []
        },
    ],

    outputs: [
        TransactionOutput {
            destination: Destination::Pubkey(
                0xd43593c715fdd31c61141abd04a99fd6
            ),
            value: 234,
        },
        TransactionOutput {
			destination: Destination::LockforStaking(
                dest: 0x2a29ab9f4878436d45299a061565714c
			),
            value: 1000,
        },
    ],
    
    timelock: ""
}
```

**TODO what is the default sighash?**
In Mintlayer, as Substrate, transanctions need to be signed before being submitted to network. Only the default sighash supported for now, so signature data contains:

- The signature hash method
- The hash of the inputs
- The hash of the outputs
- The timelock

**TODO Explain what we are showing here**
**TODO We need to document the python mintlayer crate**
**TODO what is utxos[0][0]? Utxos is a two-dimentsional array?**
**TODO I want to see the Transaction python class. What is the utxo[0][1] in the signature?**
**In the second transaction's signature, outpoints instead of outputs**
### Python

```python
from substrateinterface import Keypair
import mintlayer.utxo as utxo

client = self.nodes[0].rpc_client

alice = Keypair.create_from_uri('//Alice')
bob = Keypair.create_from_uri('//Bob')

# fetch the genesis utxo from storage
utxos = list(client.utxos_for(alice))

tx1 = utxo.Transaction(
    client,
    inputs=[
        utxo.Input(utxos[0][0]),
    ],
    outputs=[
        utxo.Output(
            value=50,
            destination=utxo.DestPubkey(bob.public_key),
            data=None
        ),
    ]).sign(alice, [utxos[0][1]])
    
    client.submit(alice, tx1)

tx2 = utxo.Transaction(
    client,
    inputs=[
        utxo.Input(tx1.outpoint(0)),
    ],
    outputs=[
        utxo.Output(
            value=30,
            destination=utxo.DestPubkey(alice.public_key),
            data=None),
        utxo.Output(
            value=20,
            destination=utxo.DestPubkey(bob.public_key),
            data=None),
    ]).sign(bob, tx1.outputs)
    
    client.submit(bob, tx2)

```

### polkadot.js

- Connect to the proper node
- Use Accounts menue
- Transfer
- [TODO] Bad Signature Error
