use node_template_runtime::{
    pallet_utxo, AccountId, BalancesConfig, GenesisConfig, PpConfig,
    Signature, SudoConfig, SystemConfig, UtxoConfig, WASM_BINARY,
    SessionConfig, StakingConfig, StakerStatus
};
use sc_network::config::MultiaddrWithPeerId;
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::H256;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use node_template_runtime::pallet_utxo::{MLT_UNIT, MLT_ORIG_SUPPLY};

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
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Keys required by pallet-session
#[derive(Clone)]
pub struct AuthorityKeys {
    controller: sr25519::Public,
    stash: sr25519::Public,
    aura_id: AuraId,
    grandpa_id: GrandpaId
}

impl  AuthorityKeys {
    fn controller_account_id(&self) -> AccountId {
        AccountPublic::from(self.controller.clone()).into_account()
    }

    fn stash_account_id(&self) -> AccountId {
        AccountPublic::from(self.stash.clone()).into_account()
    }
}

/// Generate AuthorityKeys given a seed string
pub fn authority_keys_from_seed(seed: &str) -> AuthorityKeys {
    AuthorityKeys {
        controller: get_from_seed::<sr25519::Public>(seed),
        stash: get_from_seed::<sr25519::Public>(&format!("{}//stash",seed)),
        aura_id: get_from_seed::<AuraId>(seed),
        grandpa_id: get_from_seed::<GrandpaId>(seed)
    }
}

/// Return a list of bootnodes
fn get_bootnodes() -> Vec<MultiaddrWithPeerId> {
    vec![
        "/ip4/13.59.157.140/tcp/30333/p2p/12D3KooWEEBFM1JumGXaaeimNV1UMjhoBjKMnHCeEJ4Dr5i4hLnG"
            .parse()
            .expect("Unable to parse bootnode address!"),
        "/ip4/18.222.194.251/tcp/30333/p2p/12D3KooWB11zFddP43zTSiGXvYxUuELTRigVscf9RgUpKFXeqxzF"
            .parse()
            .expect("Unable to parse bootnode address!"),
        "/ip4/3.138.108.99/tcp/30333/p2p/12D3KooWHW8LoXQhGL5aGNtvRFUUoG7UYV2qRQtwj5m57kWDwpHS"
            .parse()
            .expect("Unable to parse bootnode address!"),
    ]
}

pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
    let bootnodes = get_bootnodes();

    Ok(ChainSpec::from_genesis(
        // Name
        "Development",
        // ID
        "dev",
        ChainType::Development,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                vec![authority_keys_from_seed("Alice")],
                // Sudo account
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                // Pre-funded accounts
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                ],
                true,
            )
        },
        // Bootnodes
        bootnodes,
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
    let bootnodes = get_bootnodes();

    Ok(ChainSpec::from_genesis(
        // Name
        "Local Testnet",
        // ID
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                vec![authority_keys_from_seed("Alice")],
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
                true,
            )
        },
        // Bootnodes
        bootnodes,
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
    initial_authorities: Vec<AuthorityKeys>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> GenesisConfig
{
    const ENDOWMENT: u128 = 1 << 80;
    // minimum balance set in the runtime. check `lib.rs` of runtime module.
    const STASH: u128 = 40_000;

    // only Alice contains 400 million coins.
    let (genesis_utxos, locked_genesis) = initial_authorities
        .first()
        .map(|x| {
            let x_h256 = H256::from(x.controller.clone());
            let x_stash_h256= H256::from(x.stash.clone());

            let num_of_utxos = 5;
            let value = MLT_ORIG_SUPPLY / num_of_utxos;

            let mut initial_utxos = vec![];

            for _ in 0 .. num_of_utxos {
                initial_utxos.push( pallet_utxo::TransactionOutput::<AccountId>::new_pubkey(
                    value,
                    x_h256.clone(),
                ));
            }

            // initial authorities meaning they're also validators.
            // locking some values as a stake from validators
            let locked = pallet_utxo::TransactionOutput::<AccountId>::new_stake(
                // this is the minimum stake amount
                40_000 * MLT_UNIT,
                x_stash_h256,
                x_h256,
                vec![]
            );

            (initial_utxos, locked)
        })
        .unwrap();

    // initial_authorities also mean the starting validators in the chain.
    let stakers = initial_authorities.iter()
        .map(|auth_keys| (
            auth_keys.stash_account_id(),
            auth_keys.controller_account_id(),
            STASH,
            // the role is `validator`. See pallet-staking
            StakerStatus::Validator))
        .collect::<Vec<_>>();

    // Where aura and grandpa are initialized.
    let session_keys = initial_authorities.iter().map(|x| (
        x.stash_account_id(),
        x.stash_account_id(),
        node_template_runtime::opaque::SessionKeys {
            aura: x.aura_id.clone(),
            grandpa: x.grandpa_id.clone()
        }
    )).collect::<Vec<_>>();

    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        },
        balances: BalancesConfig {
            // Configure endowed accounts with initial balance of 1 << 80.
            balances: endowed_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect(),
        },
        // This has been initialized from the session config
        aura: Default::default(),
        // This has been initialized from the session config
        grandpa: Default::default(),
        sudo: SudoConfig {
            // Assign network admin rights.
            key: root_key,
        },
        utxo: UtxoConfig {
            genesis_utxos,
            // The # of validators set should also be the same here.
            // Currently forcing only 1 INITIAL AUTHORITY (Alice), hence only 1 locked-genesis.
            // This means only ALICE has staked her utxo.
            locked_utxos: vec![locked_genesis],
            extra_mlt_coins: 200_000_000  * MLT_UNIT,
            initial_reward_amount: 100 * MLT_UNIT,
            _marker: Default::default(),
        },
        pp: PpConfig {
            _marker: Default::default(),
        },
        session: SessionConfig {
            keys: session_keys
        },
        staking: StakingConfig {
            // provide 5 more slots. TODO: what's the max slots?
            validator_count: initial_authorities.len() as u32 + 5u32,
            // TODO: what's the actual count?
            // The # of validators set should be the same number of locked_utxos specified in UtxoConfig.
            minimum_validator_count: initial_authorities.len() as u32,
            invulnerables: initial_authorities.iter().map(|x| x.controller_account_id()).collect(),
            slash_reward_fraction: sp_runtime::Perbill::from_percent(0), // nothing, since we're not using this at all.
            stakers,
            .. Default::default()
        }
    }
}
