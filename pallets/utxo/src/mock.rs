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
use pallet_utxo::TransactionOutput;
use pp_api::ProgrammablePoolApi;

use frame_support::{dispatch::Vec, weights::Weight};
use frame_support::{
    parameter_types,
    sp_io::TestExternalities,
    sp_runtime::{
        testing::Header,
        traits::{BlakeTwo256, Hash, IdentityLookup},
    },
    traits::GenesisBuild,
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
    sp_std::{marker::PhantomData, vec},
    sr25519::Public,
    testing::SR25519,
    H256,
};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
pub type Block = frame_system::mocking::MockBlock<Test>;

pub const ALICE_PHRASE: &str =
    "news slush supreme milk chapter athlete soap sausage put clutch what kitten";

pub fn genesis_utxo() -> [u8; 32] {
    let keystore = KeyStore::new();
    let alice_pub_key = create_pub_key(&keystore, ALICE_PHRASE);
    let output = TransactionOutput::<u64>::new(100, H256::from(alice_pub_key));
    BlakeTwo256::hash_of(&output).into()
}

// Dummy programmable pool for testing
pub struct MockPool<T>(PhantomData<T>);

impl<T: frame_system::Config> ProgrammablePoolApi for MockPool<T> {
    type AccountId = u64;

    fn create(
        _origin: &Self::AccountId,
        _weight: Weight,
        _code: &Vec<u8>,
        _data: &Vec<u8>,
    ) -> Result<(), &'static str> {
        Ok(())
    }

    fn call(
        _caller: &Self::AccountId,
        _dest: &Self::AccountId,
        _gas_limit: Weight,
        _input_data: &Vec<u8>,
    ) -> Result<(), &'static str> {
        Ok(())
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
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Utxo: pallet_utxo::{Pallet, Call, Config<T>, Storage, Event<T>},
        Aura: pallet_aura::{Pallet, Config<T>, Storage},
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

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
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

// required by pallet_aura
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxAuthorities: u32 = 1000;
}

impl pallet_aura::Config for Test {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = MaxAuthorities;
}

impl pallet_utxo::Config for Test {
    type Event = Event;
    type Call = Call;
    type WeightInfo = crate::weights::WeightInfo<Test>;
    type ProgrammablePool = MockPool<Test>;

    fn authorities() -> Vec<H256> {
        Aura::authorities()
            .iter()
            .map(|x| {
                let r: &Public = x.as_ref();
                r.0.into()
            })
            .collect()
    }
}

fn create_pub_key(keystore: &KeyStore, phrase: &str) -> Public {
    SyncCryptoStore::sr25519_generate_new(keystore, SR25519, Some(phrase)).unwrap()
}

pub fn new_test_ext() -> TestExternalities {
    let keystore = KeyStore::new(); // a key storage to store new key pairs during testing
    let alice_pub_key = create_pub_key(&keystore, ALICE_PHRASE);

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    pallet_utxo::GenesisConfig::<Test> {
        genesis_utxos: vec![TransactionOutput::new(100, H256::from(alice_pub_key))],
        _marker: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = TestExternalities::from(t);
    ext.register_extension(KeystoreExt(std::sync::Arc::new(keystore)));
    ext
}

pub fn new_test_ext_and_keys() -> (TestExternalities, Public, Public) {
    // other random account generated with subkey
    const KARL_PHRASE: &str =
        "monitor exhibit resource stumble subject nut valid furnace obscure misery satoshi assume";

    let keystore = KeyStore::new();
    let alice_pub_key = create_pub_key(&keystore, ALICE_PHRASE);
    let karl_pub_key = create_pub_key(&keystore, KARL_PHRASE);

    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    pallet_utxo::GenesisConfig::<Test> {
        genesis_utxos: vec![TransactionOutput::new(100, H256::from(alice_pub_key))],
        _marker: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = TestExternalities::from(t);
    ext.register_extension(KeystoreExt(std::sync::Arc::new(keystore)));
    (ext, alice_pub_key, karl_pub_key)
}
