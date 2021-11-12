
# RUNNING

## Quick start 
Binaries can be found [here](https://github.com/mintlayer/core/releases).

Running a node also requires as input a chain specification.
Currently a single chain specification for the testnet is provided, and can be downloaded using curl:

```
curl --proto '=https' -sSf 								\
    https://raw.githubusercontent.com/mintlayer/core/master/assets/Testnet1Spec.json 	\
    --output Testnet1Spec.json
```
Download and run:
```
mintlayer-core 				\
    --base-path data/my_first_ml_node 	\
    --validator 			\
    --rpc-external 			\
    --rpc-methods Unsafe 		\
    --chain=Testnet1Spec.json
```

to start a node. It will automatically connect to the Mintlayer bootnodes.

## System Requirements
RAM: A bare minimum of 4GB RAM is required, but 8GB or more is recommended.

Disk space: a minimum of 40GB disk space is recommended.

## Building from source
To build from source, you will need to install a Rust development environment as well as some build dependencies depending on your platform.

### 1. Rust

This guide uses <https://rustup.rs> installer and the `rustup` tool to manage the Rust toolchain.
First install and configure `rustup`:

```bash
# Install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Configure
source ~/.cargo/env
```

Configure the Rust toolchain to default to the latest stable version, add nightly and the nightly wasm target:

```bash
rustup default stable
rustup update
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```
### 2. Build Dependencies 

#### Debian/Ubuntu {#de}
From a terminal shell, run:
```
sudo apt update
# May prompt for location information
sudo apt install -y git clang curl libssl-dev llvm libudev-dev
```
#### Arch Linux

From a terminal shell, run:
```
pacman -Syu --needed --noconfirm curl git clang
```
#### Fedora

From a terminal shell, run:
```
sudo dnf update
sudo dnf install clang curl git openssl-devel
```

#### MacOS (Intel-based)

**Note:** ARM M1-based MacOS systems are currently not supported. Users running an ARM system are advised to run Mintlayer on a virtual machine.

For Intel-based MacOs, open the Terminal application and execute the following commands:
```
# Install Homebrew if necessary https://brew.sh/
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install.sh)"

# Make sure Homebrew is up-to-date, install openssl
brew update
brew install openssl
```
#### Windows 
See [Mintlayer installation on Windows](windows_installation.md)

## Running a node
Clone the repository:

```
git clone https://github.com/mintlayer/core.git
```

or download the [zip](https://github.com/mintlayer/core/archive/refs/heads/master.zip).

Then, from the project's root directory, run
```
cargo build --release
```
to build the project.

Finally, to run a node:
```
RUST_LOG=info ./target/release/mintlayer-core 	\
    --base-path [PATH_TO_DB] 			\
    --name [NODE_NAME]     			\
    --port [P2P_PORT] 				\
    --ws-port [WEB_SOCKET_PORT]    		\
    --rpc-port [RPC_PORT] 			\
    --validator 				\
    --rpc-methods Unsafe 			\
    --chain=[CHAIN_SPEC]
```

For example,
```
RUST_LOG=info ./target/release/mintlayer-core 	\
    --base-path data/node1 			\
    --name brian 				\
    --port 30333 				\
    --ws-port 9945 				\
    --rpc-port 9933 				\
    --validator 				\
    --rpc-methods Unsafe 			\
    --chain=Testnet1Spec.json
```

Let's look at these flags in detail:

| <div style="min-width:110pt"> Flags </div> | Descriptions                                                                                                                                                                                                                                                                                                                               |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `--base-path`                              | Specifies a directory where Mintlayer should store all the data related to this chain. If the directory does not exist, it will be created for you. If other blockchain data already exists there you will get an error. Either clear the directory or choose a different one. |
| `--chain=[CHAIN_SPEC_FILE]`                            | Specifies which chain specification to use. A chain specification, or "chain spec", is a collection of configuration information that dictates which network a blockchain node will connect to, which entities it will initially communicate with, and what consensus-critical state it must have at genesis. |
| `--alice`                                  | Puts the predefined Alice keys (both for block production and finalization) in the node's keystore. Generally one should generate their own keys and insert them with an RPC call. We'll generate our own keys in a later step. This flag also makes Alice a validator.                                                                    |
| `--port 30333`                             | Specifies the port that your node will listen for p2p traffic on. `30333` is the default and this flag can be omitted if you're happy with the default. If Bob's node will run on the same physical system, you will need to explicitly specify a different port for it.                                                                   |
| `--ws-port 9945`                           | Specifies the port that your node will listen for incoming WebSocket traffic on. The default value is `9944`. This example uses a custom web socket port number (`9945`).                                                                                                                                                                  |
| `--rpc-port 9933`                          | Specifies the port that your node will listen for incoming RPC traffic on. `9933` is the default, so this parameter may be omitted.                                                                                                                                                                                                        |
| `--telemetry-url`                          | Tells the node to send telemetry data to a particular server. The one we've chosen here is hosted by Parity and is available for anyone to use. You may also host your own (beyond the scope of this article) or omit this flag entirely.                                                                                                  |
| `--validator`                              | Means that we want to participate in block production and finalization rather than just sync the network.                                                                                                                                                                                                                                  |
## Create a chain specification

In the preceding example, we used `--chain local` which is a predefined "chain spec" that has Alice and Bob specified as validators along with many other useful defaults.
In this example we will create a two-node network using our own custom chain specification. The process generalizes to more nodes in a straightforward manner.

Rather than writing our chain spec completely from scratch, we'll just make a few modifications to
the one we used before. To start, we need to export the chain spec to a file named
`customSpec.json`.

```bash
# Export the local chain spec to json
mintlayer-core build-spec --disable-default-bootnode --chain local > customSpec.json
```
**Note**: Further details about all commands and flags used here are available via `mintlayer-core --help`.

### Modify Aura authority nodes

The file we just created contains several fields, and you can learn a lot by exploring them. By far
the largest field is a single binary blob that is the Wasm binary of our runtime. It is part of what
you built earlier when you ran the `cargo build --release` command.

The portion of the file we're interested in is the `session`, which includes keys for `aura` authorities used for creating blocks, and `grandpa` authorities used for finalizing blocks. Using the [two provided demo keys](key_management.md#method-3-use-pre-generated-keys)

```json5
{
  //-- snip --
  "genesis": {
    "runtime": {
      "system": {
        //-- snip --
      },
      // --snip--
      "session": {
        "keys": [
          [
            "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
            "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY",
            {
              "aura": "5FfBQ3kwXrbdyoqLPvcXRp7ikWydXawpNs2Ceu3WwFdhZ8W4",
              "grandpa": "5G9NWJ5P9uk7am24yCKeLZJqXWW6hjuMyRJDmw4ofqxG8Js2"
            }
          ],
          [
            "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc",
            "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc",
            {
              "aura": "5EhrCtDaQRYjVbLi7BafbGpFqcMhjZJdu8eW8gy6VRXh6HDp",
              "grandpa": "5CRZoFgJs4zLzCCAGoCUUs2MRmuD5BKAh17pWtb62LMoCi9h"
            }
          ]
        ]
      },
      //-- snip --
    }
  }
}
```

All we need to do is change the authority addresses listed (currently Alice and Bob) to our own
addresses. For instructions on generating your own keys and addresses, see [key_management.md](key_management.md). The **sr25519** addresses go in the **aura**
section, and the **ed25519** addresses in the **grandpa** section. You may add as many validators as
you like. For additional context on cryptographic keys used in Mintlayer, check out
[this](https://docs.substrate.io/v3/advanced/cryptography#public-key-cryptography) documentation provided by Substrate.

 **Warning**: Validators should not share the same keys, even for learning purposes. If two validators have the same keys, they will produce conflicting blocks. A single person should create the chain spec and share the resulting `customSpecRaw.json` file with their fellow validators.

### Convert and share the raw chain spec

Once the chain spec is prepared, convert it to a "raw" chain spec. The raw chain spec contains all
the same information, but it contains the encoded storage keys that the node will use to reference
the data in its local storage. Distributing a raw spec ensures that each node will store the data at
the proper storage keys.

```bash
./target/release/mintlayer-core build-spec --chain=customSpec.json --raw --disable-default-bootnode  customSpecRaw.json
```

Finally share the `customSpecRaw.json` with your all the other validators in the network.

**Note**: Because Rust -> Wasm optimized builds aren't reproducible, each person will get a slightly different Wasm blob which will break consensus if each participant generates the file themselves.For the curious, learn more about this issue in [this blog post](https://dev.to/gnunicorn/hunting-down-a-non-determinism-bug-in-our-rust-wasm-build-4fk1).

## Firewall rules

The node uses TCP port 30333 for communications. This needs to be opened if you want to allow inbound connections.

### Linux-based systems
Using UFW:
```
sudo ufw allow 30333/tcp
```
Using iptables:
```
sudo iptables -A INPUT -p tcp --dport 30333 -j ACCEPT
```

### MacOS
Please consult [this](https://support.apple.com/en-gb/guide/mac-help/mh11783/mac) guide.

## Staking
See [staking.md](staking.md)

