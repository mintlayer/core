#  Blockchain developer looking to build your next project? Consider *Mintlayer*. #

Mintlayer combines combines the best features of different chains in a novel way.

At a glance, Mintlayer offers:

- Native tokenization, including NFTs and confidential tokens
- Support for WebAssembly smart contracts
- Formidable security and privacy features, by virtue of its UTXO system and the Chainscript scripting language
- Signature aggregation through BLS
- Confidential transactions
- Fully interoperablity with bitcoin and the lightning network

## Native tokenization

Mintlayer's native token, MLT, is used for staking and for paying transaction fees (**TODO** not sure about the transaction fees).
In addition, minting your own token on Mintlayer is as easy as submitting a transaction. No smart contract is required!

**TODO** Why is ease of minting important? I'd like to articulate a bit more on this 

**TODO** insert Anton's example - Ben, has he sent it yet?:

Currently, there are three types of tokens that can be issued on Mintlayer:
- **MLS-01**: "normal" tokens, akin to ERC-20 tokens on Ethererum
- **MLS-02**: confidential tokens, whose transactions are not publicly available on the blockchain (**TODO** I want to understand these better, also in relation to BLS)
- **MLS-03**: NFTs

## WebAssembly smart contracts ##

Decentralized applications of any complexity invariably require the use of smart contracts.

By supporting WebAssembly smart contracts, Mintlayer empowers blockchain developers of all backgrounds to confidently build and deploy decentralized applications.

Support for WebAssembly smart contracts means developers can code smart contracts in any language which compiles to WebAssembly.

For example, the [*ink!*](https://github.com/paritytech/ink) framerork enables smart contract development in the Rust programming language, which has seen a meteoric rise in popularity over the past years owing to its speed, safety, and rich development ecosystem. A language supporting multiple programming paradigms, Rust is also more accessible to newcomers than some purely functional languages used in the blockchain world today (**TODO** this one feels like it's trying really hard not to say "Cardano"....)

Developers versed in Ethereum will probably feel most at home coding smart contracts in Solidity. [Solang](https://github.com/hyperledger-labs/solang), a project developed by Hyperledger Labs, allows compiling Solidity to WebAssembly, thereby allowing smart contracts written in Solidity to be deployed on Mintlayer (**TODO** is this fully accurate? I read it compiles to "ewasm" which is "Ethereum-flavored" WebAssembly - are we not leaving some ugly technical details to the users here?).

## Full Interoperability with the Bitcoin ecosystem

- Mintlayer supports atomic swaps with Bitcoin without the need for an intermediary (**TODO** still supported? anything else to say here?).

## Formidable security, privacy, and performance ##

### UTXO System 

Similarly to Bitcoin, Mintlayer uses a UTXO (Unspent Transaction Output) system for transactions. This means that there is no notion of wallet (or account) at the chain level in Mintlayer as there is on blockchains such as Ethereum (**TODO** add other examples). Instead, the blockchain keeps a history of all transactions, where each transaction consists of a set of inputs and outputs. Every input to a transaction is the output of a previously executed transaction, and a transaction output is considered _unspent_ if it does not appear as an input in any transaction.

The absence of accounts at the chain level offers privacy-related advantages. For example, wallets can implement logic allowing an end user to generate a different destination address for each transaction, while the wallet takes care of determining the user's balance by scanning the blockchain for all unspent transaction outputs and determining which outputs belong to the user. Using a different destination address for each transaction renders it far more difficult to link transactions to a specific users.

Furthermore, the UTXO system also makes it relatively simple to _mix_ transactions. Commonly, this is done by means of CoinJoin, a process in which multiple inputs from different sources feed into a single transaction with several outputs having different destinations (**TODO** does this describe coinjoin or mixing more generally?). Although all inputs and outputs are stil publicly visible, it is not possible to tie any individual source address to outputs that ended up at a specific destination.

From a resource management perspective, allowing multiple source and destination addresses within a single transaction improves performance and saves space on the blockchain. (**TODO** is this true? elaborate).

Each Mintlayer block references a bitcoin block. (**TODO** what are the implications of this? What kind of attacks does this prevent? Does this give us anything else under the category of interoperablity? Still true after ditching substrate?)

**TODO** should we mention this downside? We support smart contracts and that's it, the outside world doesn't need to know what difficulties we went through to implement them.

The one downside to the UTXO system is that the stateless nature of a UTXO chain makes smart contract systems more difficult, it is for this reason Mintlayer will use programmable pools as a smart contract abstraction to an account based system.

### Chainscript

Mintlayer implements Chainscript, its own scripting language and a superset Bitcoin script. Much like Bitcoin script, Chainscript allows customization of spending conditions on funds transferred from one user to another, and can also be used for simple smart contracts.

For example, suppose Alice wants to send Bob some money provided he is able to produce a secret password picked by Alice. Alice wants to be able to take the funds back if Bob is unable or unwilling to produce the password within 2 days. In Chainscript, these conditions are expressed by:

```
OP_IF
  OP_SHA256 <SHA256_OF_PASSWORD> OP_EQUALVERIFY <Bob_PK> OP_CHECKSIG
OP_ELSE
  <T+2days> OP_CHECKLOCKTIMEVERIFY OP_DROP <Alice_PK> OP_CHECKSIG
OP_ENDIF

```
where `<T+2days>` where  represents the Unix timestamp two days from now.

This script can be redeemed by Bob:
```
<Bob_SIG> <PASSWORD> 1
```

or by Alice, after two days have elapsed:
```
<Alice_SIG> 0
```

In addition to offering simplicity, Chainscript eliminates entire classes of security issues. For example, the absence of loops in Chainscript renders DoS (Denial of Service) attacks impossible.

Furthermore, the stack-based execution model of Chainscript ensures that the time and processing resources necessary to execute a script is proportional to the size of the script. As the maximum valid size for a script is bounded, so are the resources needed to execute it. In this way, the need for gas fees is eliminated in the case of simple (Chainscript) smart contracts.

## Signature aggregation through BLS
A significant part of every transaction in a utxo system consists of the sender's signature. For example, in Bitcoin's ECDSA, the signature makes up around 1/3 of the entire transaction.

Signature aggregation is a technique which dramatically reduces the space occupied by transaction signatures on the blockchain. After a validator has selected the transactions for the next block, it uses all of the transaction signatrues as input to a _signature aggregation scheme_ (in Mintlayer's case, BLS) in order to compute a single "aggregated signature" (of the same size as the individual transaction signatures). Only the aggregated signature is stored in the block, and other validators can use it to verify all transactions in that block.

In this way, signature aggregation enables more transactions within a block of a given size. The advantages of this are manifold. The most direct benefit is storage space saved in the long run, which in in turn improves the speed of onboarding new nodes onto the chain. Having more transactions per block also makes it easier for any given transaction to be selected for the next block, which results in lower transaction processing times, and thus lower transaction fees.

## Confidential Transactions
