use sp_core::{Pair, Public, sr25519, H512};
use node_template_runtime::{
	AccountId, AuraConfig, BalancesConfig, GenesisConfig, GrandpaConfig, SessionConfig,
	SudoConfig, SystemConfig, WASM_BINARY, Signature, Balances,
	pallet_utxo, UtxoConfig};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{Verify, IdentifyAccount, BlakeTwo256, Hash};
use sc_service::ChainType;
use pallet_atomic_swap::BalanceSwapAction;

use sp_core::H256;
use frame_benchmarking::frame_support::pallet_prelude::Encode;

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


pub fn get_pair<TPublic: Public>(seed: &str) -> TPublic::Pair {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")

}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AuraId, GrandpaId) {
	(
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_from_seed::<AuraId>(s),
		get_from_seed::<GrandpaId>(s),
	)
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
			],
			// Sudo account
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			// Pre-funded accounts
			vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			],
			vec![
				get_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<sr25519::Public>("Bob")
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
				get_from_seed::<sr25519::Public>("Bob")
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
	initial_authorities: Vec<(AccountId, AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	endowed_utxos: Vec<sr25519::Public>,
	_enable_println: bool,
) -> GenesisConfig {

	let mut gen_utxo:H256 = H256::zero();
	// only Alice contains 400 million coins.
	let genesis= endowed_utxos.first().map(|x| {
		// may need to create a const variable to represent 1_000 and 100_000_000
		let x = pallet_utxo::TransactionOutput::new(
			1_000 * 100_000_000 * 400_000_000 as pallet_utxo::Value,
			H256::from_slice(x.as_slice())
		);
		gen_utxo = BlakeTwo256::hash_of(&x);

		println!("CARLA CARLA ALICE:{:?}", gen_utxo);

		x
	}).unwrap();


	 let pair = get_pair::<sr25519::Public>("Alice");

	// let genesis_utxo = H256::from([
	// 	81, 21, 116, 75, 236, 124, 214, 180, 35, 127, 81,
	// 	208, 154, 106, 21, 216, 89, 10, 92, 139, 45, 15,
	// 	227, 227, 206, 59, 82, 197, 34, 147, 181, 76]
	// );
	// 0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48
	let bob_h256 = H256::from([
		142, 175, 4, 21, 22, 135, 115, 99, 38, 201, 254, 161,
		126, 37, 252, 82, 135, 97, 54, 147, 201, 18, 144, 156,
		178, 38, 170, 71, 148, 242, 106, 72]
	);
	let bob_pub_key = sr25519::Public::from_h256(bob_h256.clone());
	let tx = pallet_utxo::Transaction{
		inputs: vec![pallet_utxo::TransactionInput::new(gen_utxo, H512::zero())],
		outputs: vec![pallet_utxo::TransactionOutput::new(50, bob_h256)]
	};
	let tx = tx.encode();
	let sig = pair.sign(&tx);
	let h512_sig = H512::from(sig);
	println!("CARLA CARLA h512: {:?}", h512_sig);


	// A generates a random proof. Keep it secret.
	let proof: [u8; 2] = [4, 2];
	// The hashed proof is the blake2_256 hash of the proof. This is public.
	let hashed_proof = BlakeTwo256::hash_of(&proof);
	println!("hashed_proof: {:?}",hashed_proof);

	let swap = pallet_atomic_swap::BalanceSwapAction::<AccountId,Balances>::new(50);
	let hashed_swap = BlakeTwo256::hash_of(&swap);
	println!("hashed swap: {:?}",hashed_swap);



	GenesisConfig {
		frame_system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k|(k, 1 << 60)).collect(),
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
					x.0.clone(),
					x.0.clone(),
					node_template_runtime::opaque::SessionKeys {
						aura: x.1.clone(),
						grandpa: x.2.clone()
					}
				)
			})
				.collect::<Vec<_>>()

		},

		pallet_utxo: UtxoConfig {
			genesis_utxos: vec![genesis],
			_marker: Default::default()
		}
	}
}
