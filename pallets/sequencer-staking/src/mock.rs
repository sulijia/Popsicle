use crate as pallet_sequencer_staking;
use crate::SEQUENCER_LOCK_ID;
use frame_support::{
	derive_impl,
	pallet_prelude::ConstU32,
	parameter_types,
	traits::{
		fungibles::Inspect,
		tokens::{Fortitude, Preservation},
		AsEnsureOriginWithArg, Everything, Hooks, LockIdentifier,
	},
	weights::constants::RocksDbWeight,
	PalletId,
};
use frame_system as system;
use frame_system::pallet_prelude::BlockNumberFor;
use popsicle_runtime::POPS;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, Perbill,
};
use std::marker::PhantomData;

pub type AccountId = u64;
pub type Balance = u128;
pub type AssetId = u32;
pub type BlockNumber = BlockNumberFor<Test>;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		SequencerStaking: pallet_sequencer_staking,
		SequencerGrouping: pallet_sequencer_grouping,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 4];
	type MaxLocks = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type RuntimeHoldReason = ();
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type MaxFreezes = ();
	type RuntimeFreezeReason = ();
}

parameter_types! {
	pub const AssetDeposit: Balance = 1;
	pub const AssetAccountDeposit: Balance = 10;
	pub const MetadataDepositBase: Balance = 1;
	pub const MetadataDepositPerByte: Balance = 1;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const RemoveItemsLimit: u32 = 5;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type WeightInfo = ();
	type CallbackHandle = ();
	type Extra = ();
	type RemoveItemsLimit = RemoveItemsLimit;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_sequencer_grouping::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Randomness = pallet_sequencer_grouping::SimpleRandomness<Self>;
	type MaxGroupSize = ConstU32<5u32>;
	type MaxGroupNumber = ConstU32<10u32>;
}

parameter_types! {
	pub const MinBlocksPerRound: u32 = 3;
	pub const MaxOfflineRounds: u32 = 1;
	pub const LeaveCandidatesDelay: u32 = 2;
	pub const CandidateBondLessDelay: u32 = 2;
	pub const LeaveDelegatorsDelay: u32 = 2;
	pub const RevokeDelegationDelay: u32 = 2;
	pub const DelegationBondLessDelay: u32 = 2;
	pub const RewardPaymentDelay: u32 = 2;
	// pub const MinSelectedCandidates: u32 = GENESIS_NUM_SELECTED_CANDIDATES;
	pub const MaxTopDelegationsPerCandidate: u32 = 4;
	pub const MaxBottomDelegationsPerCandidate: u32 = 4;
	pub const MaxDelegationsPerDelegator: u32 = 4;
	pub const MinCandidateStk: u128 = 10;
	pub const MinDelegation: u128 = 3;
	pub const RoundReward: u128 = 1 * POPS;
	pub const MaxSequencerCandidates: u32 = 200;
	pub const BTC: u32 = 0;
	pub const PalletAccount: PalletId = PalletId(*b"seqcrstk");
}

const GENESIS_BLOCKS_PER_ROUND: BlockNumber = 5;
const GENESIS_SEQUENCER_COMMISSION: Perbill = Perbill::from_percent(20);
const GENESIS_NUM_SELECTED_CANDIDATES: u32 = 5;

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	/// Interface to call Balances pallet
	type Currency = Balances;
	/// Interface to call the Assets pallet
	type Assets = Assets;
	/// Minimum round length is 1 day (60 * 60 * 24 / 6 second block times)
	type MinBlocksPerRound = MinBlocksPerRound;
	/// If a sequencer doesn't get points on this number of rounds, it is notified as inactive
	type MaxOfflineRounds = MaxOfflineRounds;
	/// Rounds before the sequencer leaving the candidates request can be executed
	type LeaveCandidatesDelay = LeaveCandidatesDelay;
	/// Rounds before the candidate bond increase/decrease can be executed
	type CandidateBondLessDelay = CandidateBondLessDelay;
	/// Rounds before the delegator exit can be executed
	type LeaveDelegatorsDelay = LeaveDelegatorsDelay;
	/// Rounds before the delegator revocation can be executed
	type RevokeDelegationDelay = RevokeDelegationDelay;
	/// Rounds before the delegator bond increase/decrease can be executed
	type DelegationBondLessDelay = DelegationBondLessDelay;
	/// Rounds before the reward is paid,
	type RewardPaymentDelay = RewardPaymentDelay;
	// /// Minimum sequencers selected per round, default at genesis and minimum forever after
	// type MinSelectedCandidates = MinSelectedCandidates;
	/// Maximum top delegations per candidate
	type MaxTopDelegationsPerCandidate = MaxTopDelegationsPerCandidate;
	/// Maximum bottom delegations per candidate
	type MaxBottomDelegationsPerCandidate = MaxBottomDelegationsPerCandidate;
	/// Maximum delegations per delegator
	type MaxDelegationsPerDelegator = MaxDelegationsPerDelegator;
	/// Minimum native token required to be locked to be a candidate
	type MinCandidateStk = MinCandidateStk;
	/// Minimum stake required to be reserved to be a delegator
	type MinDelegation = MinDelegation;
	type OnSequencerPayout = ();
	type PayoutSequencerReward = ();
	type OnInactiveSequencer = ();
	type OnNewRound = ();
	/// Interface to call the SequencerGroup pallet
	type SequencerGroup = SequencerGrouping;
	/// total rewardable native token per round
	type RoundReward = RoundReward;
	/// Account pallet id to manage rewarding native token and staked BTC
	type PalletAccount = PalletAccount;
	/// Maximum number of candidates
	type MaxCandidates = MaxSequencerCandidates;
	/// BTC asset id
	type BTC = BTC;
	type WeightInfo = ();
}

pub(crate) struct ExtBuilder {
	// endowed accounts with native balances
	balances: Vec<(AccountId, Balance)>,
	// [sequencer]
	sequencers: Vec<AccountId>,
	// [delegator, sequencer, delegation_amount]
	delegations: Vec<(AccountId, AccountId, Balance)>,
	// endowed accounts with BTC balances,
	asset_accounts: Vec<(AssetId, AccountId, Balance)>,
	// create assets: [asset_id, owner, is_sufficient, min_balance]
	assets: Vec<(AssetId, AccountId, bool, Balance)>,
	// asset metadata: id, name, symbol, decimals
	asset_metas: Vec<(AssetId, Vec<u8>, Vec<u8>, u8)>,
}

impl Default for ExtBuilder {
	fn default() -> ExtBuilder {
		ExtBuilder {
			balances: vec![],
			delegations: vec![],
			sequencers: vec![],
			asset_accounts: vec![],
			assets: vec![],
			asset_metas: vec![],
		}
	}
}

impl ExtBuilder {
	pub(crate) fn with_balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub(crate) fn with_candidates(mut self, sequencers: Vec<AccountId>) -> Self {
		self.sequencers = sequencers;
		self
	}

	pub(crate) fn with_delegations(
		mut self,
		delegations: Vec<(AccountId, AccountId, Balance)>,
	) -> Self {
		self.delegations = delegations.into_iter().map(|d| (d.0, d.1, d.2)).collect();
		self
	}

	pub(crate) fn with_assets(
		mut self,
		asset_accounts: Vec<(AssetId, AccountId, Balance)>,
		assets: Vec<(AssetId, AccountId, bool, Balance)>,
		asset_metas: Vec<(AssetId, Vec<u8>, Vec<u8>, u8)>,
	) -> Self {
		self.asset_accounts = asset_accounts;
		self.assets = assets;
		self.asset_metas = asset_metas;

		self
	}

	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Test>::default()
			.build_storage()
			.expect("Frame system builds valid default genesis config");

		pallet_balances::GenesisConfig::<Test> { balances: self.balances }
			.assimilate_storage(&mut t)
			.expect("Pallet balances storage can be assimilated");

		pallet_assets::GenesisConfig::<Test> {
			assets: self.assets,
			accounts: self.asset_accounts,
			metadata: self.asset_metas,
		}
		.assimilate_storage(&mut t)
		.expect("Pallet Assets storage can be assimilated");

		pallet_sequencer_grouping::GenesisConfig::<Test> {
			group_size: 5,
			group_number: 1,
			_marker: PhantomData,
		}
		.assimilate_storage(&mut t)
		.expect("Sequencer Grouping storage can be assimilated");

		pallet_sequencer_staking::GenesisConfig::<Test> {
			candidates: self.sequencers,
			delegations: self.delegations,
			sequencer_commission: GENESIS_SEQUENCER_COMMISSION,
			blocks_per_round: GENESIS_BLOCKS_PER_ROUND as u32,
			// num_selected_candidates: GENESIS_NUM_SELECTED_CANDIDATES,
		}
		.assimilate_storage(&mut t)
		.expect("Sequencer Staking's storage can be assimilated");

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

/// Rolls forward one block. Returns the new block number.
fn roll_one_block() -> BlockNumber {
	Balances::on_finalize(System::block_number());
	System::on_finalize(System::block_number());
	System::set_block_number(System::block_number() + 1);
	System::reset_events();
	System::on_initialize(System::block_number());
	Balances::on_initialize(System::block_number());
	SequencerStaking::on_initialize(System::block_number());
	System::block_number()
}

/// Rolls to the desired block. Returns the number of blocks played.
pub(crate) fn roll_to(n: BlockNumber) -> BlockNumber {
	let mut num_blocks = 0;
	let mut block = System::block_number();
	while block < n.into() {
		block = roll_one_block();
		num_blocks += 1;
	}
	num_blocks
}

/// Rolls desired number of blocks. Returns the final block.
pub(crate) fn roll_blocks(num_blocks: u32) -> BlockNumber {
	let mut block = System::block_number();
	for _ in 0..num_blocks {
		block = roll_one_block();
	}
	block
}

/// Rolls block-by-block to the beginning of the specified round.
/// This will complete the block in which the round change occurs.
/// Returns the number of blocks played.
pub(crate) fn roll_to_round_begin(round: BlockNumber) -> BlockNumber {
	let block = (round - 1) * GENESIS_BLOCKS_PER_ROUND;
	roll_to(block)
}

/// Rolls block-by-block to the end of the specified round.
/// The block following will be the one in which the specified round change occurs.
pub(crate) fn roll_to_round_end(round: BlockNumber) -> BlockNumber {
	let block = round * GENESIS_BLOCKS_PER_ROUND - 1;
	roll_to(block)
}

/// fn to query the lock amount
pub(crate) fn query_lock_amount(account_id: u64, id: LockIdentifier) -> Option<Balance> {
	for lock in Balances::locks(&account_id) {
		if lock.id == id {
			return Some(lock.amount);
		}
	}
	None
}

#[test]
fn geneses() {
	ExtBuilder::default()
		// native token
		.with_balances(vec![(1, 1000), (2, 300)])
		// BTC token
		.with_assets(
			// endowed accounts with BTC balances
			vec![
				(0, 3, 200),
				(0, 4, 200),
				(0, 5, 200),
				(0, 6, 200),
				(0, 7, 200),
				(0, 8, 9),
				(0, 9, 4),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		// only sequencer 1 is a candidate. sequencer 2 is not a candidate
		.with_candidates(vec![1])
		// so 5 and 6 will fail to delegate, and thus cannot be a delegator
		.with_delegations(vec![(3, 1, 100), (4, 1, 100), (5, 2, 100), (6, 2, 100)])
		.build()
		.execute_with(|| {
			assert!(System::events().is_empty());

			// sequencers
			// sequencer 1 doesn't have any BTC
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&1,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				0
			);
			// sequencer 1 should have 10 native currency locked
			assert_eq!(query_lock_amount(1, SEQUENCER_LOCK_ID), Some(10));
			// sequencer 1 is a candidate
			assert!(SequencerStaking::is_candidate(&1));

			// sequencer 2 is not a candidate, so it should not have any lock
			assert_eq!(query_lock_amount(2, SEQUENCER_LOCK_ID), None);
			// sequencer 2 doesn't have any BTC
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				0
			);
			// sequencer 2 is not a candidate
			assert!(!SequencerStaking::is_candidate(&2));

			// delegators.
			// delegators 3-7 has 200 BTC. 100 BTC is delegated to sequencers, so they should have
			// 100 BTC left
			for x in 3..5 {
				assert!(SequencerStaking::is_delegator(&x));
				assert_eq!(
					<Test as crate::Config>::Assets::reducible_balance(
						<Test as crate::Config>::BTC::get(),
						&x,
						Preservation::Expendable,
						Fortitude::Polite,
					),
					100
				);
			}
			// uninvolved
			for x in 5..10 {
				assert!(!SequencerStaking::is_delegator(&x));
			}
			// no delegator staking locks
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&5,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				200
			);
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&6,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				200
			);
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&7,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				200
			);
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&8,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				9
			);
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&9,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				4
			);
		});

	ExtBuilder::default()
		.with_balances(vec![(1, 100), (2, 100), (3, 100), (4, 100), (5, 100)])
		.with_assets(
			vec![(0, 6, 100), (0, 7, 100), (0, 8, 100), (0, 9, 100), (0, 10, 100)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 2, 3, 4, 5])
		.with_delegations(vec![(6, 1, 10), (7, 1, 10), (8, 2, 10), (9, 2, 10), (10, 1, 10)])
		.build()
		.execute_with(|| {
			assert!(System::events().is_empty());

			// sequencers
			for x in 1..5 {
				assert!(SequencerStaking::is_candidate(&x));
				// sequencer x should have 10 native currency locked
				assert_eq!(query_lock_amount(x, SEQUENCER_LOCK_ID), Some(10));
				// sequencer x doesn't have any BTC
				assert_eq!(
					<Test as crate::Config>::Assets::reducible_balance(
						<Test as crate::Config>::BTC::get(),
						&x,
						Preservation::Expendable,
						Fortitude::Polite,
					),
					0
				);
			}
			// 5 is a candidate
			assert!(SequencerStaking::is_candidate(&5));
			// sequencer 5 should have 10 native currency locked
			assert_eq!(query_lock_amount(5, SEQUENCER_LOCK_ID), Some(10));
			// sequencer 5 doesn't have any BTC
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&5,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				0
			);

			// delegators
			for x in 6..11 {
				// delegator x is a delegator
				assert!(SequencerStaking::is_delegator(&x));
				// delegator x should have 90 BTC left with 10 BTC delegated
				assert_eq!(
					<Test as crate::Config>::Assets::reducible_balance(
						<Test as crate::Config>::BTC::get(),
						&x,
						Preservation::Expendable,
						Fortitude::Polite,
					),
					90
				);
			}
		});
}

#[test]
fn roll_to_round_begin_works() {
	ExtBuilder::default().build().execute_with(|| {
		// these tests assume blocks-per-round of 5, as established by GENESIS_BLOCKS_PER_ROUND
		assert_eq!(System::block_number(), 1); // we start on block 1

		let num_blocks = roll_to_round_begin(1);
		assert_eq!(System::block_number(), 1); // no-op, we're already on this round
		assert_eq!(num_blocks, 0);

		let num_blocks = roll_to_round_begin(2);
		assert_eq!(System::block_number(), 5);
		assert_eq!(num_blocks, 4);

		let num_blocks = roll_to_round_begin(3);
		assert_eq!(System::block_number(), 10);
		assert_eq!(num_blocks, 5);
	});
}

#[test]
fn roll_to_round_end_works() {
	ExtBuilder::default().build().execute_with(|| {
		// these tests assume blocks-per-round of 5, as established by GENESIS_BLOCKS_PER_ROUND
		assert_eq!(System::block_number(), 1); // we start on block 1

		let num_blocks = roll_to_round_end(1);
		assert_eq!(System::block_number(), 4);
		assert_eq!(num_blocks, 3);

		let num_blocks = roll_to_round_end(2);
		assert_eq!(System::block_number(), 9);
		assert_eq!(num_blocks, 5);

		let num_blocks = roll_to_round_end(3);
		assert_eq!(System::block_number(), 14);
		assert_eq!(num_blocks, 5);
	});
}
