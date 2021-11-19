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

use crate::chain_spec::MltKeysInfo;
use crate::cli::{Cli, Subcommand};
use crate::{chain_spec, service};
use mintlayer_runtime::{pallet_utxo, Block, TEST_NET_MLT_ORIG_SUPPLY};
use sc_cli::{ChainSpec, Role, RuntimeVersion, SubstrateCli};
use sc_network::config::MultiaddrWithPeerId;
use sc_service::PartialComponents;
use sp_core::H256;
use sp_core::{ed25519, sr25519};
use std::error::Error;
use std::time::Duration;
use ureq::Agent;

const BOOTNODE_LIST_URL: &str =
    "https://raw.githubusercontent.com/mintlayer/core/master/assets/bootnodes.json";
const HTTP_TIMEOUT: u64 = 3000;

// actual keys for the test net
const TEST_KEYS_URL: &str =
    "https://raw.githubusercontent.com/mintlayer/core/staging/assets/test_keys.json";

// used by 'dev' mode, in the functional tests.
const FUNC_TEST_KEYS_URL: &str =
    "https://raw.githubusercontent.com/mintlayer/core/staging/assets/functional_test_keys.json";

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

// Just to help translate json from the file.
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MltKeysFromFile {
    name: String,
    sr25519_public_controller: H256,
    sr25519_public_stash: H256,
    ed25519_public: H256,
}

impl MltKeysFromFile {
    fn into_mlt_keys_info(self, mlt_tokens: pallet_utxo::tokens::Value) -> MltKeysInfo {
        MltKeysInfo {
            name: self.name,
            sr25519_public_controller: sr25519::Public::from_h256(self.sr25519_public_controller),
            sr25519_public_stash: sr25519::Public::from_h256(self.sr25519_public_stash),
            ed25519_public: ed25519::Public::from_h256(self.ed25519_public),
            mlt_tokens,
        }
    }
}

/// fetching all the needed keys for the accounts.
/// # Arguments
/// * `auth_keys_url` - Provide the url location of the keys to use as genesis validators
pub fn fetch_keys(auth_keys_url: &'static str) -> Result<Vec<MltKeysInfo>, String> {
    let mut key_list: Vec<MltKeysInfo> = vec![];

    let agent: Agent = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_millis(HTTP_TIMEOUT))
        .timeout_write(Duration::from_millis(HTTP_TIMEOUT))
        .build();

    if let Ok(contents) = agent.get(auth_keys_url).call().map_err(|e| e.to_string())?.into_string()
    {
        let users: serde_json::Value =
            serde_json::from_str(&contents).map_err(|e| e.to_string())?;

        let users = users["users"].as_array().ok_or("invalid json to extract user list")?;
        let share_per_user = TEST_NET_MLT_ORIG_SUPPLY
            .checked_div(users.len() as pallet_utxo::tokens::Value)
            .ok_or("unable to share mlt orig supply evenly.")?;

        for user in users {
            let x: MltKeysFromFile =
                serde_json::from_value(user.clone()).map_err(|e| e.to_string())?;
            key_list.push(x.into_mlt_keys_info(share_per_user));
        }
        return Ok(key_list);
    }

    log::debug!("failed to get keys from a file; using dummy values to populate keys");
    Err("failed to read keys from json file".to_string())
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
            "testnet" => Box::new(chain_spec::testnet_config(fetch_keys(TEST_KEYS_URL)?)?),
            "dev" => Box::new(chain_spec::development_config(fetch_keys(
                FUNC_TEST_KEYS_URL,
            )?)?),
            "" | "local" => Box::new(chain_spec::local_testnet_config(fetch_keys(
                FUNC_TEST_KEYS_URL,
            )?)?),
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &mintlayer_runtime::VERSION
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

                if cli.testnet {
                    // for testnet, specify chain_spec to use the `testnet_config()`
                    // from the `chain_spec.rs`
                    config.chain_spec = cli.load_spec("testnet")?;
                }

                match config.role {
                    Role::Light => service::new_light(config),
                    _ => service::new_full(config),
                }
                .map_err(sc_cli::Error::Service)
            })
        }
    }
}
