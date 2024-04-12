use crate::{weights::WeightInfo, BalanceOf, RewardPoint};
use frame_support::{dispatch::PostDispatchInfo, pallet_prelude::Weight, traits::Get};
use sp_runtime::{DispatchErrorWithPostInfo, DispatchResult};

pub trait SequencerStakingInterface<AccountId> {
	fn award_points_to_sequencer(sequencer: AccountId, points: RewardPoint) -> DispatchResult;
}

pub trait OnSequencerPayout<AccountId, Balance> {
	fn on_sequencer_payout(
		for_round: crate::RoundIndex,
		sequencer_id: AccountId,
		amount: Balance,
	) -> Weight;
}
impl<AccountId, Balance> OnSequencerPayout<AccountId, Balance> for () {
	fn on_sequencer_payout(
		_for_round: crate::RoundIndex,
		_sequencer_id: AccountId,
		_amount: Balance,
	) -> Weight {
		Weight::zero()
	}
}

pub trait OnNewRound {
	fn on_new_round(round_index: crate::RoundIndex) -> Weight;
}
impl OnNewRound for () {
	fn on_new_round(_round_index: crate::RoundIndex) -> Weight {
		Weight::zero()
	}
}

/// Defines the behavior to payout the sequencer's reward.
pub trait PayoutSequencerReward<Runtime: crate::Config> {
	fn payout_sequencer_reward(
		round_index: crate::RoundIndex,
		sequencer_id: Runtime::AccountId,
		amount: BalanceOf<Runtime>,
	) -> Weight;
}

/// Defines the default behavior for paying out the sequencer's reward. The amount is directly
/// deposited into the sequencer's account.
impl<Runtime: crate::Config> PayoutSequencerReward<Runtime> for () {
	fn payout_sequencer_reward(
		_for_round: crate::RoundIndex,
		sequencer_id: Runtime::AccountId,
		amount: BalanceOf<Runtime>,
	) -> Weight {
		// transfer the reward from the staking pallet account to the sequencer
		crate::Pallet::<Runtime>::payout_reward(amount, sequencer_id);

		// 1 read: staking account
		// 2 writes: staking account and sequencer account
		<Runtime>::DbWeight::get().reads_writes(1, 2)
	}
}

pub trait OnInactiveSequencer<Runtime: crate::Config> {
	fn on_inactive_sequencer(
		sequencer_id: Runtime::AccountId,
		round: crate::RoundIndex,
	) -> Result<Weight, DispatchErrorWithPostInfo<PostDispatchInfo>>;
}

impl<Runtime: crate::Config> OnInactiveSequencer<Runtime> for () {
	fn on_inactive_sequencer(
		sequencer_id: <Runtime>::AccountId,
		_round: crate::RoundIndex,
	) -> Result<Weight, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		crate::Pallet::<Runtime>::go_offline_inner(sequencer_id)?;
		Ok(<Runtime as crate::Config>::WeightInfo::go_offline(Runtime::MaxCandidates::get()))
	}
}
