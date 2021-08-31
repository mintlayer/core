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
// Author(s): A. Altonen
// use crate as pallet_pp;
// use sp_core::H256;
// use frame_support::parameter_types;
// use sp_runtime::{
// 	traits::{BlakeTwo256, IdentityLookup}, testing::Header,
// };
// use frame_system as system;

// type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
// type Block = frame_system::mocking::MockBlock<Test>;

// // Configure a mock runtime to test the pallet.
// frame_support::construct_runtime!(
// 	pub enum Test where
// 		Block = Block,
// 		NodeBlock = Block,
// 		UncheckedExtrinsic = UncheckedExtrinsic,
// 	{
// 		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
// 		PpModule: pallet_pp::{Pallet, Call, Storage, Event<T>},
// 	}
// );

// parameter_types! {
// 	pub const BlockHashCount: u64 = 250;
// 	pub const SS58Prefix: u8 = 42;
// }

// impl system::Config for Test {
// 	type BaseCallFilter = frame_support::traits::AllowAll;
// 	type BlockWeights = ();
// 	type BlockLength = ();
// 	type DbWeight = ();
// 	type Origin = Origin;
// 	type Call = Call;
// 	type Index = u64;
// 	type BlockNumber = u64;
// 	type Hash = H256;
// 	type Hashing = BlakeTwo256;
// 	type AccountId = u64;
// 	type Lookup = IdentityLookup<Self::AccountId>;
// 	type Header = Header;
// 	type Event = Event;
// 	type BlockHashCount = BlockHashCount;
// 	type Version = ();
// 	type PalletInfo = PalletInfo;
// 	type AccountData = ();
// 	type OnNewAccount = ();
// 	type OnKilledAccount = ();
// 	type SystemWeightInfo = ();
// 	type SS58Prefix = SS58Prefix;
// 	type OnSetCode = ();
// }

// impl pallet_pp::Config for Test {
// 	type Event = Event;
// }

// parameter_types! {
//     pub TombstoneDeposit: Balance = deposit(
//         1,
//         <pallet_contracts::Pallet<Runtime>>::contract_info_size()
//     );
//     pub DepositPerContract: Balance = TombstoneDeposit::get();
//     pub const DepositPerStorageByte: Balance = deposit(0, 1);
//     pub const DepositPerStorageItem: Balance = deposit(1, 0);
//     pub RentFraction: Perbill = Perbill::from_rational(1u32, 30 * DAYS);
//     pub const SurchargeReward: Balance = 150 * MILLICENTS;
//     pub const SignedClaimHandicap: u32 = 2;
//     pub const MaxValueSize: u32 = 16 * 1024;

//     // The lazy deletion runs inside on_initialize.
//     pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
//     BlockWeights::get().max_block;

//     // The weight needed for decoding the queue should be less or equal than a fifth
//     // of the overall weight dedicated to the lazy deletion.
//     pub DeletionQueueDepth: u32 = ((DeletionWeightLimit::get() / (
//         <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
//         <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
//     )) / 5) as u32;

//     pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
// }

// impl pallet_contracts::Config for Runtime {
//     type Time = Timestamp;
//     type Randomness = RandomnessCollectiveFlip;
//     type Currency = Balances;
//     type Event = Event;
//     type RentPayment = ();
//     type SignedClaimHandicap = SignedClaimHandicap;
//     type TombstoneDeposit = TombstoneDeposit;
//     type DepositPerContract = DepositPerContract;
//     type DepositPerStorageByte = DepositPerStorageByte;
//     type DepositPerStorageItem = DepositPerStorageItem;
//     type RentFraction = RentFraction;
//     type SurchargeReward = SurchargeReward;
//     type WeightPrice = pallet_transaction_payment::Pallet<Self>;
//     type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
//     type ChainExtension = ();
//     type DeletionQueueDepth = DeletionQueueDepth;
//     type DeletionWeightLimit = DeletionWeightLimit;
//     type Call = Call;
//     /// The safest default is to allow no calls at all.
//     ///
//     /// Runtimes should whitelist dispatchables that are allowed to be called from contracts
//     /// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
//     /// change because that would break already deployed contracts. The `Call` structure itself
//     /// is not allowed to change the indices of existing pallets, too.
//     type CallFilter = DenyAll;
//     type Schedule = Schedule;
//     type CallStack = [pallet_contracts::Frame<Self>; 31];
// }

// // Build genesis storage according to the mock runtime.
// pub fn new_test_ext() -> sp_io::TestExternalities {
// 	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
// }
