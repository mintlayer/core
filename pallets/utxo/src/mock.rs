// Copyright (c) 2021 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://spdx.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Author(s): C. Yap
use crate as pallet_utxo;
use pallet_utxo::staking::StakingHelper;
use pallet_utxo::TransactionOutput;
use pp_api::ProgrammablePoolApi;

use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_support::{dispatch::Vec, weights::Weight};
use frame_support::{
    parameter_types,
    sp_io::TestExternalities,
    sp_runtime::{
        testing::Header,
        traits::{BlakeTwo256, Hash, IdentityLookup},
        Percent,
    },
    traits::GenesisBuild,
};
use frame_system::Config as SysConfig;
use sp_core::{
    sp_std::{cell::RefCell, collections::btree_map::BTreeMap, marker::PhantomData, vec},
    sr25519::Public,
    testing::SR25519,
    H256,
};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
pub type Block = frame_system::mocking::MockBlock<Test>;
/// An index to a block.
pub type BlockNumber = u64;
pub type AccountId = H256;

pub struct MockStaking<T: pallet_utxo::Config> {
    pub withdrawal_span: T::BlockNumber,
    pub current_block: T::BlockNumber,
    pub lock_map: BTreeMap<T::AccountId, Option<T::BlockNumber>>,
    pub ctrl_map: BTreeMap<T::AccountId, T::AccountId>,
    pub lock_ctrl_map: BTreeMap<T::AccountId, T::AccountId>,
    pub marker: PhantomData<T>,
}

thread_local! {
    pub static AUTHORITIES: RefCell<Vec<Public>> = RefCell::new(vec![]);
    pub static MOCK_STAKING: RefCell<MockStaking<Test>> = RefCell::new(MockStaking::new());
}

pub const ALICE_PHRASE: &str =
    "news slush supreme milk chapter athlete soap sausage put clutch what kitten";

pub fn genesis_utxo() -> (TransactionOutput<H256>, H256) {
    let keystore = KeyStore::new();
    let alice_pub_key = create_pub_key(&keystore, ALICE_PHRASE);
    let output = TransactionOutput::<H256>::new_pubkey(100, H256::from(alice_pub_key));
    let hash = BlakeTwo256::hash_of(&(&output, 0u64, "genesis"));
    (output, hash)
}

// Dummy programmable pool for testing
pub struct MockPool<T>(PhantomData<T>);

impl<T: SysConfig> ProgrammablePoolApi for MockPool<T> {
    type AccountId = AccountId;

    fn create(
        _origin: &Self::AccountId,
        _weight: Weight,
        _code: &Vec<u8>,
        _utxo_hash: H256,
        _utxo_value: u128,
        _data: &Vec<u8>,
    ) -> Result<(), &'static str> {
        Ok(())
    }

    fn call(
        _caller: &Self::AccountId,
        _dest: &Self::AccountId,
        _gas_limit: Weight,
        _utxo_hash: H256,
        _utxo_value: u128,
        _fund_contract: bool,
        _input_data: &Vec<u8>,
    ) -> Result<(), &'static str> {
        Ok(())
    }
}

impl MockStaking<Test> {
    fn new() -> Self {
        Self {
            withdrawal_span: 5,
            current_block: 0,
            lock_map: BTreeMap::new(),
            ctrl_map: BTreeMap::new(),
            lock_ctrl_map: BTreeMap::new(),
            marker: Default::default(),
        }
    }
}

pub fn next_block() {
    MOCK_STAKING.with(|stake_info| {
        let mut stake_info = stake_info.borrow_mut();
        stake_info.current_block += 1;
    })
}

impl<T: pallet_utxo::Config> StakingHelper<AccountId> for MockStaking<T> {
    fn get_controller_account(stash_account: &AccountId) -> Result<AccountId, &'static str> {
        MOCK_STAKING.with(|stake_info| {
            let stake_info = stake_info.borrow();

            stake_info
                .lock_ctrl_map
                .get(stash_account)
                .map(|x| *x)
                .ok_or("StashAccountNotFound")
        })
    }

    fn is_controller_account_exist(controller_account: &AccountId) -> bool {
        MOCK_STAKING.with(|stake_info| {
            let stake_info = stake_info.borrow();
            stake_info.ctrl_map.contains_key(controller_account)
        })
    }

    fn can_decode_session_key(session_key: &Vec<u8>) -> bool {
        true
    }

    fn are_funds_locked(controller_account: &AccountId) -> bool {
        MOCK_STAKING.with(|stake_info| {
            let stake_info = stake_info.borrow();

            if let Some(stash) = stake_info.ctrl_map.get(controller_account) {
                if let Some(Some(_)) = stake_info.lock_map.get(stash) {
                } else {
                    return true;
                }
            }
            false
        })
    }

    fn check_accounts_matched(controller_account: &AccountId, stash_account: &AccountId) -> bool {
        MOCK_STAKING.with(|stake_info| {
            let stake_info = stake_info.borrow();

            if let Some(stash_acc) = stake_info.ctrl_map.get(controller_account) {
                if stash_account == stash_acc {
                    return true;
                }
            }
            false
        })
    }

    fn lock_for_staking(
        stash_account: &AccountId,
        controller_account: &AccountId,
        _rotate_keys: &Vec<u8>,
        _value: u128,
    ) -> DispatchResultWithPostInfo {
        MOCK_STAKING.with(|stake_info| {
            let mut stake_info = stake_info.borrow_mut();

            if stake_info.lock_map.contains_key(stash_account) {
                Err("CANNOT STAKE. STASH ACCOUNT IS ALREADY REGISTERED.")?
            }

            if stake_info.ctrl_map.contains_key(stash_account) {
                Err("CANNOT STAKE. STASH ACCOUNT IS ACTUALLY A CONTROLLER ACCOUNT")?
            }

            if stake_info.lock_ctrl_map.contains_key(controller_account) {
                Err(pallet_utxo::Error::<T>::StashAccountAlreadyRegistered)?
            }

            stake_info.lock_map.insert(stash_account.clone(), None);
            stake_info
                .lock_ctrl_map
                .insert(stash_account.clone(), controller_account.clone());
            stake_info.ctrl_map.insert(controller_account.clone(), stash_account.clone());

            Ok(().into())
        })
    }

    fn lock_extra_for_staking(
        stash_account: &AccountId,
        _value: u128,
    ) -> DispatchResultWithPostInfo {
        MOCK_STAKING.with(|stake_info| {
            let stake_info = stake_info.borrow();

            if !stake_info.lock_map.contains_key(stash_account) {
                Err(pallet_utxo::Error::<T>::StashAccountNotFound)?
            }

            if stake_info.ctrl_map.contains_key(stash_account) {
                Err("CANNOT STAKE. STASH ACCOUNT IS ACTUALLY A CONTROLLER ACCOUNT")?
            }

            Ok(().into())
        })
    }

    fn unlock_request_for_withdrawal(stash_account: &AccountId) -> DispatchResultWithPostInfo {
        MOCK_STAKING.with(|stake_info| {
            let mut stake_info = stake_info.borrow_mut();

            if stake_info.ctrl_map.contains_key(stash_account) {
                Err("CANNOT PAUSE. STASH ACCOUNT IS ACTUALLY A CONTROLLER ACCOUNT")?
            }

            if !stake_info.lock_map.contains_key(stash_account) {
                Err(pallet_utxo::Error::<T>::StashAccountNotFound)?
            }

            if let Some(_) = stake_info.lock_map.get(stash_account).unwrap() {
                // if it has a value already, meaning a pause function was already performed.
                Err("CANNOT PAUSE AGAIN.")?
            }

            let withdrawal_block = stake_info.current_block + stake_info.withdrawal_span;
            stake_info.lock_map.insert(stash_account.clone(), Some(withdrawal_block));

            Ok(().into())
        })
    }

    fn withdraw(stash_account: &AccountId) -> DispatchResultWithPostInfo {
        MOCK_STAKING.with(|stake_info| {
            let mut stake_info = stake_info.borrow_mut();

            match stake_info.lock_map.get(stash_account) {
                Some(Some(withdrawal_block)) => {
                    if *withdrawal_block <= stake_info.current_block {
                        let ctrl_account = stake_info.lock_ctrl_map.remove(stash_account).unwrap();
                        stake_info.ctrl_map.remove(&ctrl_account);
                        stake_info.lock_map.remove(&stash_account);

                        Ok(().into())
                    } else {
                        Err("not yet time to withdraw".into())
                    }
                }
                Some(_) | None => { Err("not yet unlocked".into()) }

            }

        })
    }
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Utxo: pallet_utxo::{Pallet, Call, Config<T>, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
    pub const MinimumPeriod: u64 = 1;

    pub const MaximumBlockLength: u32 = 2 * 1024;
}

impl SysConfig for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxAuthorities: u32 = 1000;
    pub const MinimumStake: u128 = 10;
    pub const InitialReward: u128 = 100;
    pub const DefaultMinimumReward: u128 = 1;
    pub const StakeWithdrawalFee: u128 = 1;
    pub const RewardReductionPeriod: BlockNumber = 5;
    pub const RewardReductionFraction: Percent = Percent::from_percent(25);
}

impl pallet_utxo::Config for Test {
    type Event = Event;
    type Call = Call;
    type WeightInfo = crate::weights::WeightInfo<Test>;
    type ProgrammablePool = MockPool<Test>;
    type AssetId = u64;
    type RewardReductionFraction = RewardReductionFraction;
    type RewardReductionPeriod = RewardReductionPeriod;

    fn authorities() -> Vec<H256> {
        AUTHORITIES.with(|auths| {
            let auths = auths.borrow();
            auths.iter().map(|x| H256::from(x.0)).collect()
        })
    }

    type StakingHelper = MockStaking<Test>;
    type MinimumStake = MinimumStake;
    type StakeWithdrawalFee = StakeWithdrawalFee;
    type InitialReward = InitialReward;
    type DefaultMinimumReward = DefaultMinimumReward;
}

fn create_pub_key(keystore: &KeyStore, phrase: &str) -> Public {
    SyncCryptoStore::sr25519_generate_new(keystore, SR25519, Some(phrase)).unwrap()
}

pub fn alice_test_ext() -> TestExternalities {
    let keystore = KeyStore::new(); // a key storage to store new key pairs during testing
    let alice_pub_key = create_pub_key(&keystore, ALICE_PHRASE);

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    pallet_utxo::GenesisConfig::<Test> {
        genesis_utxos: vec![TransactionOutput::new_pubkey(100, H256::from(alice_pub_key))],
        locked_utxos: vec![],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = TestExternalities::from(t);
    ext.register_extension(KeystoreExt(std::sync::Arc::new(keystore)));
    ext
}

pub fn alice_test_ext_and_keys() -> (TestExternalities, Public, Public) {
    // other random account generated with subkey
    const KARL_PHRASE: &str =
        "monitor exhibit resource stumble subject nut valid furnace obscure misery satoshi assume";

    let keystore = KeyStore::new();
    let alice_pub_key = create_pub_key(&keystore, ALICE_PHRASE);
    let karl_pub_key = create_pub_key(&keystore, KARL_PHRASE);

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    pallet_utxo::GenesisConfig::<Test> {
        genesis_utxos: vec![TransactionOutput::new_pubkey(100, H256::from(alice_pub_key))],
        locked_utxos: vec![],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = TestExternalities::from(t);
    ext.register_extension(KeystoreExt(std::sync::Arc::new(keystore)));
    (ext, alice_pub_key, karl_pub_key)
}

pub fn multiple_keys_test_ext() -> (TestExternalities, Vec<(Public, H256)>) {
    const KARL_PHRASE: &str =
        "monitor exhibit resource stumble subject nut valid furnace obscure misery satoshi assume";

    const GREG_PHRASE: &str =
        "infant salmon buzz patrol maple subject turtle cute legend song vital leisure";

    const TOM_PHRASE: &str = "clip organ olive upper oak void inject side suit toilet stick narrow";

    let keystore = KeyStore::new();

    let alice_pub_key = create_pub_key(&keystore, ALICE_PHRASE);
    let karl_pub_key = create_pub_key(&keystore, KARL_PHRASE);
    let greg_pub_key = create_pub_key(&keystore, GREG_PHRASE);
    let tom_pub_key = create_pub_key(&keystore, TOM_PHRASE);

    let alice_hash = H256::from(alice_pub_key);
    let karl_hash = H256::from(karl_pub_key);
    let greg_hash = H256::from(greg_pub_key);
    let tom_hash = H256::from(tom_pub_key);

    let alice_genesis = TransactionOutput::new_pubkey(100, alice_hash.clone());
    let karl_genesis = TransactionOutput::new_pubkey(110, karl_hash.clone());
    let greg_genesis = TransactionOutput::new_pubkey(120, greg_hash.clone());
    let tom_genesis = TransactionOutput::new_pubkey(130, tom_hash.clone());

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    pallet_utxo::GenesisConfig::<Test> {
        genesis_utxos: vec![
            alice_genesis.clone(),
            karl_genesis.clone(),
            greg_genesis.clone(),
            tom_genesis.clone(),
        ],
        locked_utxos: vec![
            //  alice is the stash and tom is a controller account.
            TransactionOutput::new_lock_for_staking(10, alice_hash, tom_hash, vec![3, 1]),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = TestExternalities::from(t);
    ext.register_extension(KeystoreExt(std::sync::Arc::new(keystore)));

    MOCK_STAKING.with(|stake_info| {
        let mut stake_info = stake_info.borrow_mut();
        stake_info.lock_map.insert(alice_hash, None);
        stake_info.lock_ctrl_map.insert(alice_hash, tom_hash);
        stake_info.ctrl_map.insert(tom_hash, alice_hash);
    });

    AUTHORITIES.with(|auths| {
        let mut auths = auths.borrow_mut();
        auths.push(alice_pub_key);
    });

    (
        ext,
        vec![
            (
                alice_pub_key,
                BlakeTwo256::hash_of(&(&alice_genesis, 0u64, "genesis")),
            ),
            (
                karl_pub_key,
                BlakeTwo256::hash_of(&(&karl_genesis, 1u64, "genesis")),
            ),
            (
                greg_pub_key,
                BlakeTwo256::hash_of(&(&greg_genesis, 2u64, "genesis")),
            ),
            (
                tom_pub_key,
                BlakeTwo256::hash_of(&(&tom_genesis, 3u64, "genesis")),
            ),
        ],
    )
}
