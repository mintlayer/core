# Mintlayer core

https://www.mintlayer.org/

For a more technical introduction to Mintlayer visit [our docs](https://docs.mintlayer.org/).

A draft of the consensus paper can be found [here](https://www.mintlayer.org/docs/DSA-consensus-paper-draft.pdf).


### Rust Setup

First, complete the [basic Rust setup instructions](./docs/rust-setup.md).

### Run

Use Rust's native `cargo` command to build and launch the template node:

```sh
cargo run --release -- --dev --tmp
```

### Build

The `cargo run` command will perform an initial build. Use the following command to build the node
without launching it:

```sh
cargo build --release
```
or 

`cargo build` to build a debug version

to pure the local chain run `./target/release/mintlayer-core purge-chain --dev`

### Docs

Once the project has been built, the following command can be used to explore all parameters and
subcommands:

```sh
./target/release/mintlayer-core -h
```

You can also find docs in the docs directory and within the directory for a specific pallet or lib.


### Single-Node Development Chain

This command will start the single-node development chain with persistent state:

```bash
./target/release/mintlayer-core --dev
```

Purge the development chain's state:

```bash
./target/release/mintlayer-core purge-chain --dev
```

Start the development chain with detailed logging:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 ./target/release/mintlayer-core -lruntime=debug --dev
```

### Connect with Polkadot-JS Apps Front-end

Once the node template is running locally, you can connect it with **Polkadot-JS Apps** front-end
to interact with your chain. [Click here](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944) connecting the Apps to your local node template.

### Connect with Mintlayer's UI
TODO

### Multi-Node Local Testnet

If you want to see the multi-node consensus algorithm in action, refer to
[our Start a Private Network tutorial](https://substrate.dev/docs/en/tutorials/start-a-private-network/).

## Project Structure

### Node

- Networking: Mintlayer uses [libp2p](https://libp2p.io/) as its native networking stack for all inter-node communication.
- Bootnodes: Mintlayer has [bootnodes](https://github.com/mintlayer/core/blob/master/assets/bootnodes.json) that a new node will attempt to boot to unless a specific node is specified by the user
- Consensus: Mintlayer uses [AURA](https://docs.rs/sc-consensus-aura/0.9.0/sc_consensus_aura/) as its base consensus algorithm for the time being. There will be an update to introduce [DSA](https://www.mintlayer.org/docs/DSA-consensus-paper-draft.pdf) in the future but DSA is still in development. 
- Finality: Since we are using AURA for our consensus we currently rely on [GRANDPA](https://docs.rs/sc-finality-grandpa/0.9.0/sc_finality_grandpa/) for finality.
- Chain Spec: You can find our chain specification in [chain_spec.rs](https://github.com/mintlayer/core/blob/master/node/src/chain_spec.rs). It defines the basics of the chain such as the genesis block and the bootnodes.
- Services: [service.rs](https://github.com/mintlayer/core/blob/master/node/src/service.rs) defines the node implementation itself. It is here you'll find the consensus setup.


### Runtime

For more information on what a [runtime](https://substrate.dev/docs/en/knowledgebase/getting-started/glossary#runtime) is follow the link.

Most of Mintlayer's staking code exists in the runtime, see `staking.rs` and `lib.rs` to see that code. Other things that exist here are bits related to block production such as the block production period. The runtime must be written in no_std rust since it compiles to wasm.

### Pallets

Mintlayer relies on a host of Substrate pallets and a few Mintlayer specific pallets.

-   pp: The implementation of programmable pools on Mintlayer. Essentially wasm smart contracts
-   utxo: Mintlayer's utxo system
    
### Libs

Libs is home to code that is code that Mintlayer relies on but isn't technically a pallet. 

-   chainscript: Mintlayer's bitcoin script implementation.
-   bech32: code for handling transactions with destinations encoded using bech32

### Testing

You'll find unit tests littered throughout the codebase but the test directory is home to the functional test framework which is heavily based on Bitcoin's functional test framework. 

### Crypto
As it stands Mintlayer uses Schnorr for all crypto-related things. There is a plan to move to our BLS implementation in the near future but this, as it stands, is a work in progress.

### Contributing
[See this guide](https://github.com/mintlayer/core/CONTRIBUTING.md)

### Branches
The key branches are master and staging. Master is used for fully tested code, staging is used as the development branch. Fixes or features should be created on new branches branched from staging. A pr is then created to merge the branch into staging where it will require a review from a member of the Mintlayer team. To merge into master create a pr to merge staging to master, a review is required and CI will run. Only select people have push access to master.

### Firewall rules

The node uses TCP port 30333 for communications, this needs to be opened if you want to allow
inbound connections.

Using UFW:
`sudo ufw allow 30333/tcp`

Using iptables:
`sudo iptables -A INPUT -p tcp --dport 30333 -j ACCEPT`
