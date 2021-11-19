#  Blockchain developer looking to build your next project? Consider *Mintlayer*. #

Mintlayer combines combines the best features of different chains in a novel way.

**TODO** Order of the items?

At a glance, Mintlayer offers:

- Native tokenization, including NFTs and confidential tokens
- Support for WebAssembly smart contracts
- Signature aggregation through BLS 
- Fully interoperablity with bitcoin and the lightning network
- Formidable security and privacy features


**TODO** I think these are "lower level" and should not be mentioned in the "At a glance bullets" - Ben, what do you think? I have difficult time selling chainscript in a one-liner.

- Chainscript - Mintlayer's superset of Bitcoin script
- A UTXO based transaction model, reminiscent of Bitcoin's



## Native tokenization ##:

**TODO** How to mention the native MLT token? What is its role?

Minting your token on Mintlayer is as easy as submitting a transaction. No smart contract is required!

**TODO** Why is ease of minting important? I'd like to articulate a bit more on this 

**TODO** insert Anton's example - Ben, has he sent it yet?:

Currently, there are three types of tokens that can be issued on Mintlayer (**TODO** I think it is friendlier (more concrete) to say "types tokens" than "token standards". But maybe the novice in me is being nitpicky):
- **MLS-01**: "normal" tokens, akin to ERC-20 tokens on Ethererum
- **MLS-02**: confidential tokens (**TODO** Ben, let's have a chat about these as I do not entirely understand what they are. In particular I want to understand what it means that these tokens "do not rely on BLS for signature aggregation" - what is BLS? Let's talk about it)"
- **MLS-03**: NFTs  (**TODO** need I say more? Everyone knows about these)

## WebAssembly Smart Contracts ##

Decentralized applications of any complexity invariably require the use of smart contracts.

By supporting WebAssembly smart contracts, Mintlayer empowers blockchain developers of all backgrounds to confidently build and deploy decentralized applications.

Support for WebAssembly smart contracts means developers can code smart contracts in any language which compiles to WebAssembly.

For example, the [*ink!*](https://github.com/paritytech/ink) framerork enables smart contract development in the Rust programming language, which has seen a meteoric rise in popularity over the past years owing to its speed, safety, and rich development ecosystem. A language supporting multiple programming paradigms, Rust is also more accessible to newcomers than some purely functional languages used in the blockchain world today (**TODO** this one feels like it's trying really hard not to say "Cardano"....)

Developers versed in Ethereum will probably feel most at home coding smart contracts in Solidity. [Solang](https://github.com/hyperledger-labs/solang), a project developed by Hyperledger Labs, allows compiling Solidity to WebAssembly, thereby allowing smart contracts written in Solidity to be deployed on Mintlayer (**TODO** is this fully accurate? I read it compiles to "ewasm" which is "Ethereum-flavored" WebAssembly - are we not leaving some ugly technical details to the users here?).

## Full Interoperability with the Bitcoin ecosystem

**TODO** What does interoperablity/compatibility with Bitcoin really mean? 

- Mintlayer supports atomic swaps with Bitcoin

## Formidable security, privacy, and performance inspired by Bitcoin ##

For transactions Mintlayer uses a UTXO (Unspent Transaction Output) system, reminiscent of Bitcoin's. This means that there is no notion of "account" in Mintlayer as there is in blockchains such as Ethereum (**TODO** add other examples so it's not just Ethereum). Instead, the blockchain keeps a database of transactions with source and destination addresses (**TODO** this feels to me poorly phrased from a technical standpoint - Ben help). This comes with several advantages. From a privacy perspective, this allows to derive unique destination addresses for each transactions, which makes chain analysis much more difficult (**TODO** maybe include an example of what we mean by chain analysis). From a resource management perspective, this allows source and destination addresses to be included in a single transaction, which improves performance and saves space on the blockchain. (**TODO** is this true?).
Each Mintlayer block references a bitcoin block. (**TODO** what are the implications of this? What kind of attacks does this prevent? Does this give us anything else under the category of interoperablity?)

In addition, Mintlayer uses implements Chainscript, its own implementation of Bitcoin script. Here is an example of [**insert description**] using Chainscript:

**TODO** insert one of the examples Lukas gave me

**TODO** is it accurate to  call Chainscript our IMPLEMENTATION of Bitcoin script? There are some differences, I understood from Lukas. But he also said that in principle any Bitcoin script should be able to execute properly on the Chainscript interpretrer.

In addition to offering simplicity, Chainscript eliminates entire classes of security issues. For example, the absence of loops in Chainscript renders DoS (Denial of Service) attacks impossible.

Furthermore, the stack-based execution model of Chainscript ensures that the time and processing resources necessary to execute a script is proportional to the size of the script, which in turn removes the need for gas fees (**TODO** couldn't someone in theory spam us with a really long Chainscript script?).
