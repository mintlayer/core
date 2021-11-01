// This file is part of Substrate.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::cli::{Cli, Subcommand};
use crate::{chain_spec::{self, get_from_seed}, service};
use node_template_runtime::{Block, pallet_utxo::MLT_UNIT};
use sc_cli::{ChainSpec, Role, RuntimeVersion, SubstrateCli};
use sc_network::config::MultiaddrWithPeerId;
use sc_service::PartialComponents;
use sp_core::{sr25519, ed25519};
use std::error::Error;
use std::time::Duration;
use ureq::Agent;
use sp_core::H256;
use crate::chain_spec::MltKeysInfo;

const BOOTNODE_LIST_URL: &str =
    "https://raw.githubusercontent.com/mintlayer/core/master/assets/bootnodes.json";
const HTTP_TIMEOUT: u64 = 3000;

//TODO: change this path to the exact one we need
const KEYS_URL:&str =
    "https://raw.githubusercontent.com/mintlayer/core/rewards_and_staking/assets/test_keys.json";

/// Fetch an up-to-date list of bootnodes from Github
fn fetch_bootnode_list() -> Result<Vec<MultiaddrWithPeerId>, Box<dyn Error>> {
    let agent: Agent = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_millis(HTTP_TIMEOUT))
        .timeout_write(Duration::from_millis(HTTP_TIMEOUT))
        .build();

    let response: String = agent.get(BOOTNODE_LIST_URL).call()?.into_string()?;
    let json: serde_json::Value = serde_json::from_str(&response)?;

    let nodes = json["nodes"].as_array().ok_or("Invalid JSON")?;
    let mut parsed_nodes: Vec<MultiaddrWithPeerId> = Vec::new();

    for node in nodes.iter() {
        parsed_nodes.push(node.as_str().ok_or("Invalid JSON")?.parse()?);
    }

    Ok(parsed_nodes)
}


#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MltKeysFromFile {
     name: String,
     sr25519_public_controller: H256,
     sr25519_public_stash: H256,
     ed25519_public: H256,
     mlt_coins: u128
}

impl From<MltKeysFromFile> for MltKeysInfo {
    fn from(x: MltKeysFromFile) -> Self {
        Self {
            name: x.name,
            sr25519_public_controller: sr25519::Public::from_h256(x.sr25519_public_controller),
            sr25519_public_stash: sr25519::Public::from_h256(x.sr25519_public_stash),
            ed25519_public: ed25519::Public::from_h256(x.ed25519_public),
            mlt_coins: x.mlt_coins
        }
    }
}


/// fetching all the needed keys for the accounts.
pub fn fetch_keys() ->  Result<Vec<MltKeysInfo>, String>{
    let mut key_list:Vec<MltKeysInfo> = vec![];

    let agent: Agent = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_millis(HTTP_TIMEOUT))
        .timeout_write(Duration::from_millis(HTTP_TIMEOUT))
        .build();

    if let Ok(contents) = agent.get(KEYS_URL).call().map_err(|e| e.to_string())?.into_string(){
        let users:serde_json::Value = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
        for user in users["users"].as_array().ok_or("invalid json to extract user list")? {
            let mut x:MltKeysFromFile = serde_json::from_value(user.clone()).map_err(|e| e.to_string())?;
            x.mlt_coins = x.mlt_coins * MLT_UNIT;
            key_list.push(x.into());
        };
    } else {
        log::info!("failed to get keys from a file; using dummy values to populate keys");
        //TODO: dummy values, in case the file doesn't exist.
       impl MltKeysInfo {
           fn new(seed:&str, mlt_coins:u128) -> MltKeysInfo {
               MltKeysInfo {
                   name: seed.to_string(),
                   sr25519_public_controller: get_from_seed::<sr25519::Public>(seed),
                   sr25519_public_stash: get_from_seed::<sr25519::Public>(&format!("{}//stash",seed)),
                   ed25519_public: get_from_seed::<ed25519::Public>(seed),
                   mlt_coins: mlt_coins * MLT_UNIT
               }
           }
       }

        key_list.push(MltKeysInfo::new("Alice", 399_600_000_000));
        key_list.push(MltKeysInfo::new("Bob", 100_000_000));
        key_list.push(MltKeysInfo::new("Charlie", 100_000_000));
        key_list.push(MltKeysInfo::new("Dave", 100_000_000));
        key_list.push(MltKeysInfo::new("Eve", 100_000_000));
    }

    Ok(key_list)
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Mintlayer Node".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        "Mintlayer".into()
    }

    fn support_url() -> String {
        "https://github.com/mintlayer/core/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2021
    }

    fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(chain_spec::development_config(fetch_keys()?)?),
            "" | "local" => Box::new(chain_spec::local_testnet_config(fetch_keys()?)?),
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &node_template_runtime::VERSION
    }
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, backend), task_manager))
            })
        }
        Some(Subcommand::Benchmark(cmd)) => {
            if cfg!(feature = "runtime-benchmarks") {
                let runner = cli.create_runner(cmd)?;

                runner.sync_run(|config| cmd.run::<Block, service::ExecutorDispatch>(config))
            } else {
                Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
                    .into())
            }
        }
        None => {
            let runner = cli.create_runner(&cli.run)?;
            runner.run_node_until_exit(|mut config| async move {
                // if dev chainspec is not used, fetch an up-to-date bootnode list from Github
                match config.chain_spec.id() {
                    "dev" => {}
                    _ => match fetch_bootnode_list() {
                        Ok(mut bootnodes) => config.network.boot_nodes.append(&mut bootnodes),
                        Err(e) => log::error!("Failed to update bootnode list: {:?}", e),
                    },
                }

                let keys = fetch_keys()?;
                log::info!("fetch all keys: {:?}",keys);

                match config.role {
                    Role::Light => service::new_light(config),
                    _ => service::new_full(config),
                }
                .map_err(sc_cli::Error::Service)
            })
        }
    }
}
