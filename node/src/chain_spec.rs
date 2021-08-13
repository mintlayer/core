use sp_core::{Pair, Public, sr25519, H256};
use node_template_runtime::{
	AccountId, BalancesConfig, GenesisConfig, SessionConfig,
	StakingConfig, CouncilConfig, ElectionsConfig,
	SudoConfig, SystemConfig, WASM_BINARY, Signature,
	pallet_utxo, UtxoConfig, StakerStatus,
	constants::currency::DOLLARS
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{Verify, IdentifyAccount};
use sc_service::ChainType;


use sp_runtime::Perbill;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}


struct AuthKeys {
	account_id: AccountId,
	stash_acount_id: AccountId,
	aura_id: AuraId,
	grandpa_id: GrandpaId
}

/// Generate an Aura authority key.
fn authority_keys_from_seed(seed: &str) -> AuthKeys {
	AuthKeys {
		account_id: get_account_id_from_seed::<sr25519::Public>(seed),
		stash_acount_id: get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash",seed)),
		aura_id: get_from_seed::<AuraId>(seed),
		grandpa_id: get_from_seed::<GrandpaId>(seed)
	}
}

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || testnet_genesis(
			wasm_binary,
			// Initial PoA authorities
			vec![
				authority_keys_from_seed("Alice"),
				authority_keys_from_seed("Bob"),
				authority_keys_from_seed("Charlie"),
				authority_keys_from_seed("Dave")
			],
			// Sudo account
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			// Pre-funded accounts
			vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
				get_account_id_from_seed::<sr25519::Public>("Dave"),
				get_account_id_from_seed::<sr25519::Public>("Eve"),
				get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
				get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
				get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			],
			vec![
				get_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<sr25519::Public>("Bob"),
				get_from_seed::<sr25519::Public>("Charlie"),
				get_from_seed::<sr25519::Public>("Dave")
			],
			true,
		),
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || testnet_genesis(
			wasm_binary,
			// Initial PoA authorities
			vec![
				authority_keys_from_seed("Alice"),
				authority_keys_from_seed("Bob"),
				authority_keys_from_seed("Charlie"),
				authority_keys_from_seed("Dave")
			],
			// Sudo account
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			// Pre-funded accounts
			vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
				get_account_id_from_seed::<sr25519::Public>("Dave"),
				get_account_id_from_seed::<sr25519::Public>("Eve"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
				get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
				get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
			],
			vec![
				get_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<sr25519::Public>("Bob"),
				get_from_seed::<sr25519::Public>("Charlie"),
				get_from_seed::<sr25519::Public>("Dave"),
				get_from_seed::<sr25519::Public>("Eve"),
				get_from_seed::<sr25519::Public>("Ferdie")
			],
			true,
		),
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<AuthKeys>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	endowed_utxos: Vec<sr25519::Public>,
	_enable_println: bool,
) -> GenesisConfig {

	const ENDOWMENT: u128 = 10_000_000 * DOLLARS;
	const STASH: u128 = ENDOWMENT / 1000;

	let num_endowed_accounts = endowed_accounts.len();

	// only Alice contains 400 million coins.
	let genesis= endowed_utxos.first().map(|x| {
		// may need to create a const variable to represent 1_000 and 100_000_000
		pallet_utxo::TransactionOutput::new(
			1_000 * 100_000_000 * 400_000_000 as pallet_utxo::Value,
			H256::from_slice(x.as_slice())
		)
	}).unwrap();

	let stakers = initial_authorities.iter()
		.map(|auth_keys|
			(auth_keys.stash_acount_id.clone(), auth_keys.account_id.clone(),STASH, StakerStatus::Validator))
		.collect::<Vec<_>>();

	GenesisConfig {
		frame_system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k|(k, ENDOWMENT)).collect(),
		},
		pallet_aura: Default::default(),
		pallet_grandpa:Default::default(),
		pallet_sudo: SudoConfig {
			// Assign network admin rights.
			key: root_key,
		},

		pallet_session: SessionConfig {
			keys: initial_authorities.iter().map(|x|{
				(
					x.stash_acount_id.clone(),
					x.account_id.clone(),
					node_template_runtime::opaque::SessionKeys {
						aura: x.aura_id.clone(),
						grandpa: x.grandpa_id.clone()
					}
				)
			})
				.collect::<Vec<_>>()

		},

		pallet_staking: StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| {
				x.account_id.clone()
			}).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			.. Default::default()
		},

		pallet_elections_phragmen: ElectionsConfig {
			members: endowed_accounts.iter()
				.take( (num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect()
		},

		pallet_collective_Instance1: CouncilConfig::default(),

		pallet_utxo: UtxoConfig {
			genesis_utxos: vec![genesis],
			_marker: Default::default()
		}
	}
}
