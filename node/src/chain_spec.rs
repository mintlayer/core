use mintlayer_runtime::{
    pallet_utxo, AccountId, BalancesConfig, GenesisConfig, PpConfig, SessionConfig, Signature,
    StakerStatus, StakingConfig, SudoConfig, SystemConfig, UtxoConfig, MINIMUM_STAKE,
    NUM_OF_VALIDATOR_SLOTS, WASM_BINARY,
};
use sc_network::config::MultiaddrWithPeerId;
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, H256};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

pub type AccountPublic = <Signature as Verify>::Signer;

/// Holds information about keys needed for the accounts
#[derive(Default, Debug, Clone)]
pub struct MltKeysInfo {
    pub name: String,
    pub sr25519_public_controller: sr25519::Public,
    pub sr25519_public_stash: sr25519::Public,
    pub ed25519_public: sp_core::ed25519::Public,
    pub mlt_tokens: pallet_utxo::tokens::Value,
}

impl MltKeysInfo {
    fn controller_account_id(&self) -> AccountId {
        AccountPublic::from(self.sr25519_public_controller).into_account()
    }

    fn stash_account_id(&self) -> AccountId {
        AccountPublic::from(self.sr25519_public_stash).into_account()
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

pub fn testnet_config(endowed_accounts: Vec<MltKeysInfo>) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
    let bootnodes = get_bootnodes();

    let mut validators = endowed_accounts.clone();
    // remove alice from the validators list.
    validators.remove(0);
    // setting bob as the sudo user.
    let sudo = validators.first().cloned().ok_or("endowed accounts is empty")?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Mintlayer_TestNet",
        // ID
        "mlt_test_net",
        ChainType::Custom("MLTTestNet".into()), //TODO: I don't think this worked at all, it still goes to local_testnet
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities;
                validators.clone(),
                // Sudo account
                sudo.controller_account_id(),
                // Pre-funded accounts;
                endowed_accounts.clone(),
                // Pre-fund all accounts in the pallet-balance
                endowed_accounts.clone(),
            )
        },
        // Bootnodes
        bootnodes,
        // Telemetry
        None,
        // Protocol ID
        "MintlayerTestV0".into(),
        // Properties
        None,
        // Extensions
        None,
    ))
}

pub fn development_config(endowed_accounts: Vec<MltKeysInfo>) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
    let bootnodes = get_bootnodes();

    // only Alice has sudo powers
    let sudo = endowed_accounts.first().cloned().ok_or("endowed accounts is empty")?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Development",
        // ID
        "dev",
        ChainType::Development,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities; 2
                endowed_accounts.iter().cloned().take(2).collect(),
                // Sudo account
                sudo.controller_account_id(),
                // Pre-funded accounts; only the first 2 are funded. This is important for
                // the functional tests.
                endowed_accounts.iter().cloned().take(2).collect(),
                // Pre-fund all accounts in the pallet-balance
                endowed_accounts.clone(),
            )
        },
        // Bootnodes
        bootnodes,
        // Telemetry
        None,
        // Protocol ID
        "MintlayerDev".into(),
        // Properties
        None,
        // Extensions
        None,
    ))
}

pub fn local_testnet_config(endowed_accounts: Vec<MltKeysInfo>) -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
    let bootnodes = get_bootnodes();

    // only Alice
    let sudo = endowed_accounts.first().cloned().ok_or("endowed accounts is empty")?;

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
                endowed_accounts.iter().cloned().take(2).collect(),
                // Sudo account; the first one, being Alice
                sudo.controller_account_id(),
                // Pre-funded utxos for the ff. accounts
                endowed_accounts.clone(),
                // Pre-funded all the accounts in the pallet-balance
                endowed_accounts.clone(),
            )
        },
        // Bootnodes
        bootnodes,
        // Telemetry
        None,
        // Protocol ID
        "MintlayerTestLocal".into(),
        // Properties
        None,
        // Extensions
        None,
    ))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<MltKeysInfo>,
    root_key: AccountId,
    endowed_utxos: Vec<MltKeysInfo>,
    endowed_accounts: Vec<MltKeysInfo>,
) -> GenesisConfig {
    //TODO: clean up this code
    // Endowment for the pallet-balances.
    // Bigger endowment means security from all fees or costs generated by doing multiple actions;
    // actions which are not charged when dealing with the utxo system.
    // An example is the first time staking; that involves 3 steps; but in the utxo system, it's only 1.
    const ENDOWMENT: u128 = 1 << 90;

    let (locked_utxos, stakers, session_keys) = initial_authorities.iter().fold(
        (
            vec![], // the utxos locked
            vec![], // the stakers: (<stash_account_id>, <controller_account_id>, <stash_amount>, <role>)
            vec![], // session keys: (<account_id>, <validator_id a.k.a. account_id>, <runtime_defined_keys>). See Pallet-Session
        ),
        |(mut locked_utxos, mut stakers, mut session_keys), auth_keys| {
            // initial authorities meaning they're also validators.
            // locking some values as a stake from validators
            locked_utxos.push(
                pallet_utxo::TransactionOutput::<AccountId>::new_lock_for_staking(
                    // this is the minimum stake amount
                    MINIMUM_STAKE,
                    auth_keys.stash_account_id(),
                    auth_keys.controller_account_id(),
                    vec![],
                ),
            );

            // to initialize the pallet-staking
            stakers.push((
                auth_keys.stash_account_id(),
                auth_keys.controller_account_id(),
                // minimum balance set in the runtime. check `lib.rs` of runtime module.
                MINIMUM_STAKE,
                // the role is `validator`. See pallet-staking
                StakerStatus::Validator,
            ));

            // Where aura and grandpa are initialized.
            session_keys.push((
                auth_keys.stash_account_id(),
                auth_keys.stash_account_id(),
                mintlayer_runtime::opaque::SessionKeys {
                    aura: AuraId::from(auth_keys.sr25519_public_controller),
                    grandpa: GrandpaId::from(auth_keys.ed25519_public),
                },
            ));

            (locked_utxos, stakers, session_keys)
        },
    );

    let genesis_utxos: Vec<pallet_utxo::TransactionOutput<AccountId>> =
        endowed_utxos.into_iter().fold(vec![], |mut genesis_utxos, info| {
            // share tokens between the controller and the stash accounts
            let shared_tokens = info.mlt_tokens / 2;

            // add tokens for the controller account
            genesis_utxos.push(pallet_utxo::TransactionOutput::<AccountId>::new_pubkey(
                shared_tokens,
                H256::from(info.sr25519_public_controller),
            ));

            // add token for the stash_account
            genesis_utxos.push(pallet_utxo::TransactionOutput::<AccountId>::new_pubkey(
                shared_tokens,
                H256::from(info.sr25519_public_stash),
            ));
            genesis_utxos
        });

    let balances = endowed_accounts.iter().fold(vec![], |mut acc, info| {
        acc.push((info.controller_account_id(), ENDOWMENT));
        acc.push((info.stash_account_id(), ENDOWMENT));
        acc
    });

    GenesisConfig {
        system: SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        },
        balances: BalancesConfig {
            // Configure endowed accounts
            balances,
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
            // This should be the same as what's set as the initial authorities
            locked_utxos,
            // initial_reward_amount: 100 * MLT_UNIT
        },
        pp: PpConfig {
            _marker: Default::default(),
        },
        session: SessionConfig { keys: session_keys },
        staking: StakingConfig {
            validator_count: NUM_OF_VALIDATOR_SLOTS,
            // The # of validators set should be the same number of locked_utxos specified in UtxoConfig.
            minimum_validator_count: 1,
            invulnerables: initial_authorities.iter().map(|x| x.controller_account_id()).collect(),
            slash_reward_fraction: sp_runtime::Perbill::from_percent(0), // nothing, since we're not using this at all.
            stakers,
            ..Default::default()
        },
    }
}
