#  Blockchain developer looking to build your next project? Consider *Mintlayer*. #

Mintlayer combines combines the best features of different chains in a novel way.

**TODO** Order of the items?

At a glance, Mintlayer offers:

- Native tokenization, including NFTs and confidential tokens
- Support for WebAssembly smart contracts
- Signature aggregation through BLS 
- Fully interoperablity with bitcoin and the lightning network
- Formidable security and privacy features

## Native tokenization ##:

Mintlayer's native token, MLT, is used for staking and for paying transaction fees.

In addition, minting your own token on Mintlayer is as easy as submitting a transaction. No smart contract is required!

**TODO** Why is ease of minting important? I'd like to articulate a bit more on this 

**TODO** insert Anton's example - Ben, has he sent it yet?:

Currently, there are three types of tokens that can be issued on Mintlayer:
- **MLS-01**: "normal" tokens, akin to ERC-20 tokens on Ethererum
- **MLS-02**: confidential tokens, whose transactions are not publicly available on the blockchain (**TODO** I want to understand these better, also in relation to BLS)
- **MLS-03**: NFTs

## WebAssembly Smart Contracts ##

Decentralized applications of any complexity invariably require the use of smart contracts.

By supporting WebAssembly smart contracts, Mintlayer empowers blockchain developers of all backgrounds to confidently build and deploy decentralized applications.

Support for WebAssembly smart contracts means developers can code smart contracts in any language which compiles to WebAssembly.

For example, the [*ink!*](https://github.com/paritytech/ink) framerork enables smart contract development in the Rust programming language, which has seen a meteoric rise in popularity over the past years owing to its speed, safety, and rich development ecosystem. A language supporting multiple programming paradigms, Rust is also more accessible to newcomers than some purely functional languages used in the blockchain world today (**TODO** this one feels like it's trying really hard not to say "Cardano"....)

Developers versed in Ethereum will probably feel most at home coding smart contracts in Solidity. [Solang](https://github.com/hyperledger-labs/solang), a project developed by Hyperledger Labs, allows compiling Solidity to WebAssembly, thereby allowing smart contracts written in Solidity to be deployed on Mintlayer (**TODO** is this fully accurate? I read it compiles to "ewasm" which is "Ethereum-flavored" WebAssembly - are we not leaving some ugly technical details to the users here?).

## Full Interoperability with the Bitcoin ecosystem

- Mintlayer supports atomic swaps with Bitcoin without the need for an intermediary (**TODO** still supported? anything else to say here?).

## Formidable security, privacy, and performance ##

## UTXO System 

For transactions Mintlayer uses a UTXO (Unspent Transaction Output) system, reminiscent of Bitcoin's. This means that there is no notion of "account" in Mintlayer as there is in blockchains such as Ethereum (**TODO** add other examples so it's not just Ethereum). Instead, the blockchain keeps a database of transactions with source and destination addresses (**TODO** this feels to me poorly phrased from a technical standpoint - Ben help). This comes with several advantages. From a privacy perspective, this allows to derive unique destination addresses for each transaction, which makes chain analysis much more difficult (**TODO** maybe include an example of what we mean by chain analysis). From a resource management perspective, this allows multiple source and destination addresses to be included in a single transaction, which improves performance and saves space on the blockchain. (**TODO** is this true?).
Each Mintlayer block references a bitcoin block. (**TODO** what are the implications of this? What kind of attacks does this prevent? Does this give us anything else under the category of interoperablity? Still true after ditching substrate?)

## Chainscript

In addition, Mintlayer implements Chainscript, its own scripting language and a superset Bitcoin script. Much like Bitcoin script, Chainscript allows customization of spending conditions on funds transferred from one user to another, and can also be used for simple smart contracts.

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

## BLS Signatures
