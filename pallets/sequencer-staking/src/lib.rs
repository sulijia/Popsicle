#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod delegation_requests;
pub mod set;
pub mod traits;
pub mod types;
pub mod weights;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

pub use traits::*;
pub use types::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use crate::{
		delegation_requests::{CancelledScheduledRequest, DelegationAction, ScheduledRequest},
		set::BoundedOrderedSet,
		traits::*,
		types::*,
		WeightInfo,
	};
	use frame_support::{
		dispatch::DispatchResultWithPostInfo,
		pallet_prelude::*,
		traits::{
			fungibles,
			fungibles::{Inspect, Mutate},
			tokens::{Fortitude, Preservation, WithdrawReasons},
			Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency,
			ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use pallet_sequencer_grouping::SequencerGroup;
	use sp_runtime::{
		traits::{AccountIdConversion, Saturating, Zero},
		DispatchErrorWithPostInfo, Perbill,
	};
	use sp_std::{collections::btree_map::BTreeMap, prelude::*};
	// Round index type
	pub type RoundIndex = u32;
	// Reward points type
	pub type RewardPoint = u32;

	// AccountId type
	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	// Native token balance type
	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	/// Assets module balance type
	pub type AssetBalanceOf<T> =
		<<T as Config>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
	/// Id type for assets
	pub type AssetIdOf<T> =
		<<T as Config>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

	/// Sequencer lock identifier
	pub const SEQUENCER_LOCK_ID: LockIdentifier = *b"sequencr";

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The currency type
		type Currency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;
		/// Type to access the Assets Pallet.
		type Assets: fungibles::Inspect<Self::AccountId> + fungibles::Mutate<Self::AccountId>;
		/// Minimum number of blocks per round
		#[pallet::constant]
		type MinBlocksPerRound: Get<u32>;
		/// If a sequencer doesn't produce any block on this number of rounds, it is notified as
		/// inactive. This value must be less than or equal to RewardPaymentDelay.
		#[pallet::constant]
		type MaxOfflineRounds: Get<u32>;
		/// Number of rounds that candidates remain bonded before exit request is executable
		#[pallet::constant]
		type LeaveCandidatesDelay: Get<RoundIndex>;
		/// Number of rounds candidate requests to decrease self-bond must wait to be executable
		#[pallet::constant]
		type CandidateBondLessDelay: Get<RoundIndex>;
		/// Number of rounds that delegators remain bonded before exit request is executable
		#[pallet::constant]
		type LeaveDelegatorsDelay: Get<RoundIndex>;
		/// Number of rounds that delegations remain bonded before revocation request is executable
		#[pallet::constant]
		type RevokeDelegationDelay: Get<RoundIndex>;
		/// Number of rounds that delegation less requests must wait before executable
		#[pallet::constant]
		type DelegationBondLessDelay: Get<RoundIndex>;
		/// Number of rounds after which block authors are rewarded
		#[pallet::constant]
		type RewardPaymentDelay: Get<RoundIndex>;
		/// Minimum number of selected candidates every round
		// #[pallet::constant]
		// type MinSelectedCandidates: Get<u32>;
		/// Maximum top delegations counted per candidate
		#[pallet::constant]
		type MaxTopDelegationsPerCandidate: Get<u32>;
		/// Maximum bottom delegations (not counted) per candidate
		#[pallet::constant]
		type MaxBottomDelegationsPerCandidate: Get<u32>;
		/// Maximum delegations per delegator
		#[pallet::constant]
		type MaxDelegationsPerDelegator: Get<u32>;
		/// Minimum native token locked required for any account to be a sequencer candidate
		#[pallet::constant]
		type MinCandidateStk: Get<BalanceOf<Self>>;
		/// Minimum stake for any registered on-chain account to delegate
		#[pallet::constant]
		type MinDelegation: Get<AssetBalanceOf<Self>>;
		/// Handler to notify the runtime when a sequencer is paid.
		/// If you don't need it, you can specify the type `()`.
		type OnSequencerPayout: OnSequencerPayout<Self::AccountId, BalanceOf<Self>>;
		/// Handler to distribute a sequencer's reward.
		/// To use the default implementation of minting rewards, specify the type `()`.
		type PayoutSequencerReward: PayoutSequencerReward<Self>;
		/// Handler to notify the runtime when a sequencer is inactive.
		/// The default behavior is to mark the sequencer as offline.
		/// If you need to use the default implementation, specify the type `()`.
		type OnInactiveSequencer: OnInactiveSequencer<Self>;
		/// Handler to notify the runtime when a new round begin.
		/// If you don't need it, you can specify the type `()`.
		type OnNewRound: OnNewRound;
		// An interface to call the sequencer-group pallet
		type SequencerGroup: pallet_sequencer_grouping::SequencerGroup<
			AccountIdOf<Self>,
			BlockNumberFor<Self>,
		>;
		/// The native token reward for a round
		#[pallet::constant]
		type RoundReward: Get<BalanceOf<Self>>;
		/// The account that issues the rewards to sequencers
		#[pallet::constant]
		type PalletAccount: Get<PalletId>;
		/// Maximum candidates
		#[pallet::constant]
		type MaxCandidates: Get<u32>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: crate::weights::WeightInfo;
		/// BTC asset id
		#[pallet::constant]
		type BTC: Get<AssetIdOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	/// Commission percent taken off of rewards for all sequencers
	pub(crate) type SequencerCommission<T: Config> = StorageValue<_, Perbill, ValueQuery>;

	// #[pallet::storage]
	// /// The total candidates selected every round
	// pub(crate) type TotalSelected<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	/// Current round index and next round scheduled transition
	pub type Round<T: Config> = StorageValue<_, RoundInfo<BlockNumberFor<T>>, ValueQuery>;

	#[pallet::storage]
	/// Get delegator state associated with an account if account is delegating else None
	pub(crate) type DelegatorState<T: Config> = StorageMap<
		_,
		Twox64Concat,
		AccountIdOf<T>,
		Delegator<AccountIdOf<T>, AssetBalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	/// Get sequencer candidate info associated with an account if account is candidate else None
	pub(crate) type CandidateInfo<T: Config> = StorageMap<
		_,
		Twox64Concat,
		AccountIdOf<T>,
		CandidateMetadata<AssetBalanceOf<T>>,
		OptionQuery,
	>;

	/// Stores outstanding delegation requests per sequencer.
	#[pallet::storage]
	pub(crate) type DelegationScheduledRequests<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		BoundedVec<
			ScheduledRequest<AccountIdOf<T>, AssetBalanceOf<T>>,
			AddGet<T::MaxTopDelegationsPerCandidate, T::MaxBottomDelegationsPerCandidate>,
		>,
		ValueQuery,
	>;

	#[pallet::storage]
	/// Top delegations for sequencer candidate
	pub(crate) type TopDelegations<T: Config> = StorageMap<
		_,
		Twox64Concat,
		AccountIdOf<T>,
		Delegations<AccountIdOf<T>, AssetBalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	/// Bottom delegations for sequencer candidate
	pub(crate) type BottomDelegations<T: Config> = StorageMap<
		_,
		Twox64Concat,
		AccountIdOf<T>,
		Delegations<AccountIdOf<T>, AssetBalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	/// The sequencer candidates selected for the current round
	pub(crate) type SelectedCandidates<T: Config> =
		StorageValue<_, BoundedVec<AccountIdOf<T>, T::MaxCandidates>, ValueQuery>;

	#[pallet::storage]
	/// Total capital locked by this staking pallet
	pub(crate) type Total<T: Config> = StorageValue<_, AssetBalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	/// The pool of sequencer candidates, each with their total backing stake
	pub(crate) type CandidatePool<T: Config> = StorageValue<
		_,
		BoundedOrderedSet<Bond<AccountIdOf<T>, AssetBalanceOf<T>>, T::MaxCandidates>,
		ValueQuery,
	>;

	#[pallet::storage]
	/// Snapshot of sequencer delegation stake at the start of the round
	pub type AtStake<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		RoundIndex,
		Twox64Concat,
		AccountIdOf<T>,
		SequencerSnapshot<AccountIdOf<T>, AssetBalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	/// Delayed payouts
	pub type DelayedPayouts<T: Config> =
		StorageMap<_, Twox64Concat, RoundIndex, DelayedPayout<BalanceOf<T>>, OptionQuery>;

	#[pallet::storage]
	/// Total points awarded to sequencers for block production in the round
	pub type Points<T: Config> = StorageMap<_, Twox64Concat, RoundIndex, RewardPoint, ValueQuery>;

	#[pallet::storage]
	/// Points for each sequencer per round
	pub type AwardedPts<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		RoundIndex,
		Twox64Concat,
		AccountIdOf<T>,
		RewardPoint,
		ValueQuery,
	>;

	#[pallet::storage]
	/// Killswitch to enable/disable marking offline feature.
	pub type EnableMarkingOffline<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// Initialize balance and register all as sequencers: `(sequencer AccountId, balance
		/// Amount)`
		pub candidates: Vec<AccountIdOf<T>>,
		/// Initialize balance and make delegations:
		/// `(delegator AccountId, sequencer AccountId, delegation Amount)`
		pub delegations: Vec<(AccountIdOf<T>, AccountIdOf<T>, AssetBalanceOf<T>)>,
		/// Default fixed percent a sequencer takes off the top of due rewards
		pub sequencer_commission: Perbill,
		/// Default number of blocks in a round
		pub blocks_per_round: u32,
		// /// Number of selected candidates every round. Cannot be lower than
		// MinSelectedCandidates pub num_selected_candidates: u32,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				candidates: vec![],
				delegations: vec![],
				sequencer_commission: Default::default(),
				blocks_per_round: 1u32,
				// num_selected_candidates: T::MinSelectedCandidates::get(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			assert!(self.blocks_per_round > 0, "Blocks per round must be > 0");

			let mut candidate_count = 0u32;
			// Initialize the candidates
			for candidate in &self.candidates {
				let lock_amount = T::MinCandidateStk::get();
				assert!(
					T::Currency::free_balance(&candidate) >= lock_amount,
					"Account does not have enough balance to be locked as a candidate."
				);

				if let Err(error) = <Pallet<T>>::join_candidates(
					T::RuntimeOrigin::from(Some(candidate.clone()).into()),
					candidate_count,
				) {
					log::warn!("Join candidates failed in genesis with error {:?}", error);
				} else {
					candidate_count = candidate_count.saturating_add(1u32);
				}
			}

			let mut col_delegator_count: BTreeMap<T::AccountId, u32> = BTreeMap::new();
			let mut del_delegation_count: BTreeMap<T::AccountId, u32> = BTreeMap::new();

			// Initialize the delegations
			for &(ref delegator, ref target, balance) in &self.delegations {
				let delegator_balance = T::Assets::reducible_balance(
					T::BTC::get(),
					&delegator,
					Preservation::Expendable,
					Fortitude::Polite,
				);

				assert!(
					delegator_balance >= balance,
					"Account does not have enough balance to place delegation."
				);
				let cd_count =
					if let Some(x) = col_delegator_count.get(target) { *x } else { 0u32 };
				let dd_count =
					if let Some(x) = del_delegation_count.get(delegator) { *x } else { 0u32 };

				if let Err(error) = <Pallet<T>>::delegate(
					T::RuntimeOrigin::from(Some(delegator.clone()).into()),
					target.clone(),
					balance,
					cd_count,
					dd_count,
				) {
					log::warn!("Delegate failed in genesis with error {:?}", error);
				} else {
					if let Some(x) = col_delegator_count.get_mut(target) {
						*x = x.saturating_add(1u32);
					} else {
						col_delegator_count.insert(target.clone(), 1u32);
					};
					if let Some(x) = del_delegation_count.get_mut(delegator) {
						*x = x.saturating_add(1u32);
					} else {
						del_delegation_count.insert(delegator.clone(), 1u32);
					};
				}
			}

			// Set sequencer commission to default config
			<SequencerCommission<T>>::put(self.sequencer_commission);

			// // Set total selected candidates to value from config
			// assert!(
			// 	self.num_selected_candidates >= T::MinSelectedCandidates::get(),
			// 	"{:?}",
			// 	Error::<T>::CannotSetBelowMin
			// );
			// assert!(
			// 	self.num_selected_candidates <= T::MaxCandidates::get(),
			// 	"{:?}",
			// 	Error::<T>::CannotSetAboveMaxCandidates
			// );

			// <TotalSelected<T>>::put(self.num_selected_candidates);

			// Choose top TotalSelected sequencer candidates
			let (_, v_count, _, _total_staked) = <Pallet<T>>::select_top_candidates(1u32);

			// Start Round 1 at Block 0, with snapshot at 3/4 of the round
			let snapshot_time_point = self.blocks_per_round * 3 / 4;
			let round: RoundInfo<BlockNumberFor<T>> =
				RoundInfo::new(1u32, Zero::zero(), self.blocks_per_round, snapshot_time_point);
			<Round<T>>::put(round);

			<Pallet<T>>::deposit_event(Event::NewRound {
				starting_block: Zero::zero(),
				round: 1u32,
				selected_sequencers_number: v_count,
			});
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Started new round.
		NewRound {
			starting_block: BlockNumberFor<T>,
			round: RoundIndex,
			selected_sequencers_number: u32,
		},
		/// Candidates selected for the next round.
		CandidatesSelected {
			round: RoundIndex,
			selected_sequencers_number: u32,
			total_balance: AssetBalanceOf<T>,
		},
		/// Account joined the set of sequencer candidates.
		JoinedSequencerCandidates { account: AccountIdOf<T>, amount_locked: BalanceOf<T> },
		/// Candidate selected for sequencers. Total Exposed Amount includes all delegations.
		SequencerChosen {
			round: RoundIndex,
			sequencer_account: AccountIdOf<T>,
			total_exposed_amount: AssetBalanceOf<T>,
		},
		/// Candidate requested to decrease a self bond.
		CandidateBondLessRequested {
			candidate: AccountIdOf<T>,
			amount_to_decrease: AssetBalanceOf<T>,
			execute_round: RoundIndex,
		},
		/// Candidate has increased a self bond.
		CandidateBondedMore {
			candidate: AccountIdOf<T>,
			amount: AssetBalanceOf<T>,
			new_total_bond: AssetBalanceOf<T>,
		},
		/// Candidate has decreased a self bond.
		CandidateBondedLess {
			candidate: AccountIdOf<T>,
			amount: AssetBalanceOf<T>,
			new_bond: AssetBalanceOf<T>,
		},
		/// Candidate temporarily leave the set of sequencer candidates without unbonding.
		CandidateWentOffline { candidate: AccountIdOf<T> },
		/// Candidate rejoins the set of sequencer candidates.
		CandidateBackOnline { candidate: AccountIdOf<T> },
		/// Candidate has requested to leave the set of candidates.
		CandidateScheduledExit {
			exit_allowed_round: RoundIndex,
			candidate: AccountIdOf<T>,
			scheduled_exit: RoundIndex,
		},
		/// Cancelled request to leave the set of candidates.
		CancelledCandidateExit { candidate: AccountIdOf<T> },
		/// Cancelled request to decrease candidate's bond.
		CancelledCandidateBondLess {
			candidate: AccountIdOf<T>,
			amount: AssetBalanceOf<T>,
			execute_round: RoundIndex,
		},
		/// Candidate has left the set of candidates.
		CandidateLeft {
			ex_candidate: AccountIdOf<T>,
			unlocked_amount: AssetBalanceOf<T>,
			new_total_amt_locked: AssetBalanceOf<T>,
		},
		/// Delegator requested to decrease a bond for the sequencer candidate.
		DelegationDecreaseScheduled {
			delegator: AccountIdOf<T>,
			candidate: AccountIdOf<T>,
			amount_to_decrease: AssetBalanceOf<T>,
			execute_round: RoundIndex,
		},
		// Delegation increased.
		DelegationIncreased {
			delegator: AccountIdOf<T>,
			candidate: AccountIdOf<T>,
			amount: AssetBalanceOf<T>,
			in_top: bool,
		},
		// Delegation decreased.
		DelegationDecreased {
			delegator: AccountIdOf<T>,
			candidate: AccountIdOf<T>,
			amount: AssetBalanceOf<T>,
			in_top: bool,
		},
		/// Delegator requested to revoke delegation.
		DelegationRevocationScheduled {
			round: RoundIndex,
			delegator: AccountIdOf<T>,
			candidate: AccountIdOf<T>,
			scheduled_exit: RoundIndex,
		},
		/// Delegator has left the set of delegators.
		DelegatorLeft { delegator: AccountIdOf<T>, unstaked_amount: AssetBalanceOf<T> },
		/// Delegation revoked.
		DelegationRevoked {
			delegator: AccountIdOf<T>,
			candidate: AccountIdOf<T>,
			unstaked_amount: AssetBalanceOf<T>,
		},
		/// Delegation kicked.
		DelegationKicked {
			delegator: AccountIdOf<T>,
			candidate: AccountIdOf<T>,
			unstaked_amount: AssetBalanceOf<T>,
		},
		/// Cancelled request to change an existing delegation.
		CancelledDelegationRequest {
			delegator: AccountIdOf<T>,
			cancelled_request: CancelledScheduledRequest<AssetBalanceOf<T>>,
			sequencer: AccountIdOf<T>,
		},
		/// New delegation (increase of the existing one).
		Delegation {
			delegator: AccountIdOf<T>,
			locked_amount: AssetBalanceOf<T>,
			candidate: AccountIdOf<T>,
			delegator_position: DelegatorAdded<AssetBalanceOf<T>>,
		},
		/// Delegation from candidate state has been remove.
		DelegatorLeftCandidate {
			delegator: AccountIdOf<T>,
			candidate: AccountIdOf<T>,
			unstaked_amount: AssetBalanceOf<T>,
			total_candidate_staked: AssetBalanceOf<T>,
		},
		/// Paid the account (delegator or sequencer) the balance as liquid rewards.
		Rewarded { account: AccountIdOf<T>, rewards: BalanceOf<T> },
		/// Set total selected candidates to this value.
		TotalSelectedSet { old: u32, new: u32 },
		/// Set sequencer commission to this value.
		SequencerCommissionSet { old: Perbill, new: Perbill },
		/// Set blocks per round
		BlocksPerRoundSet {
			current_round: RoundIndex,
			first_block: BlockNumberFor<T>,
			old: u32,
			new: u32,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		DelegatorDNE,
		CandidateDNE,
		DelegationDNE,
		DelegatorExists,
		CandidateExists,
		CandidateBondBelowMin,
		InsufficientBalance,
		DelegatorBondBelowMin,
		DelegationBelowMin,
		AlreadyOffline,
		AlreadyActive,
		CandidateAlreadyLeaving,
		CandidateNotLeaving,
		CandidateCannotLeaveYet,
		CannotGoOnlineIfLeaving,
		ExceedMaxDelegationsPerDelegator,
		AlreadyDelegatedCandidate,
		CannotSetBelowMin,
		RoundLengthMustBeGreaterThanTotalSelectedSequencers,
		NoWritingSameValue,
		TooLowCandidateCountWeightHintJoinCandidates,
		TooLowCandidateCountWeightHintCancelLeaveCandidates,
		TooLowCandidateCountToLeaveCandidates,
		TooLowDelegationCountToDelegate,
		TooLowCandidateDelegationCountToDelegate,
		TooLowCandidateDelegationCountToLeaveCandidates,
		PendingCandidateRequestsDNE,
		PendingCandidateRequestAlreadyExists,
		PendingCandidateRequestNotDueYet,
		PendingDelegationRequestDNE,
		PendingDelegationRequestAlreadyExists,
		PendingDelegationRequestNotDueYet,
		CannotDelegateLessThanOrEqualToLowestBottomWhenFull,
		PendingDelegationRevoke,
		TooLowSequencerCountToNotifyAsInactive,
		CannotBeNotifiedAsInactive,
		CandidateLimitReached,
		CannotSetAboveMaxCandidates,
		MarkingOfflineNotEnabled,
		CurrentRoundTooLow,
		TransferFailed,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut weight = <T as Config>::WeightInfo::base_on_initialize();

			// change round if necessary
			let mut round = <Round<T>>::get();
			if round.should_update(n.into()) {
				// mutate round
				round.update(n.into());
				// notify that new round begin
				weight = weight.saturating_add(T::OnNewRound::on_new_round(round.current));
				// pay all stakers for T::RewardPaymentDelay rounds ago
				weight = weight.saturating_add(Self::prepare_staking_payouts(round));

				// get the total number of selected candidates
				let sequencer_count = <SelectedCandidates<T>>::decode_len().unwrap_or(0) as u32;
				// account for SelectedCandidates reads and writes
				weight = weight.saturating_add(
					T::DbWeight::get().reads_writes(sequencer_count.into(), sequencer_count.into()),
				);

				// start next round
				<Round<T>>::put(round);
				// account for Round write
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(0, 1));

				Self::deposit_event(Event::NewRound {
					starting_block: round.first,
					round: round.current,
					selected_sequencers_number: sequencer_count,
				});
			} else {
				weight = weight.saturating_add(Self::handle_delayed_payouts(round.current));
			}

			// snapshot the current data for the next round if necessary
			if round.should_snapshot(n.into()) {
				// select top sequencer candidates for next round
				let (extra_weight, sequencer_count, _delegation_count, total_staked) =
					Self::select_top_candidates(round.current.saturating_add(1));
				weight = weight.saturating_add(extra_weight);

				let next_round = round.current.saturating_add(1);
				Self::deposit_event(Event::CandidatesSelected {
					round: next_round,
					selected_sequencers_number: sequencer_count,
					total_balance: total_staked,
				});

				let selected_candidates =
					<SelectedCandidates<T>>::get().into_iter().collect::<Vec<_>>();
				let next_round_first_block = round.first.saturating_add(round.length.into());

				// trigger sequencer grouping
				let _ = T::SequencerGroup::trigger_group(
					selected_candidates,
					next_round_first_block,
					next_round,
				);
			}

			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Charge the caller Native token for the reward account
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::charge_reward_account())]
		pub fn charge_reward_account(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let acc = ensure_signed(origin)?;

			T::Currency::transfer(
				&acc,
				&Self::account_id(),
				amount,
				ExistenceRequirement::KeepAlive,
			)?;

			Ok(().into())
		}
		/// Set the total number of sequencer candidates selected per round
		/// - changes are not applied until the next snapshot time point
		// #[pallet::call_index(1)]
		// #[pallet::weight(<T as Config>::WeightInfo::set_total_selected())]
		// pub fn set_total_selected(origin: OriginFor<T>, new: u32) -> DispatchResultWithPostInfo {
		// 	frame_system::ensure_root(origin)?;
		// 	ensure!(new >= T::MinSelectedCandidates::get(), Error::<T>::CannotSetBelowMin);
		// 	ensure!(new <= T::MaxCandidates::get(), Error::<T>::CannotSetAboveMaxCandidates);
		// 	// let old = SequencerGroup::total_selected();
		// 	let old = SequencerGroup::total_selected();
		// 	ensure!(old != new, Error::<T>::NoWritingSameValue);
		// 	ensure!(
		// 		new < <Round<T>>::get().length,
		// 		Error::<T>::RoundLengthMustBeGreaterThanTotalSelectedSequencers,
		// 	);
		// 	<TotalSelected<T>>::put(new);
		// 	Self::deposit_event(Event::TotalSelectedSet { old, new });
		// 	Ok(().into())
		// }

		/// Set the commission for all sequencers
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::set_sequencer_commission())]
		pub fn set_sequencer_commission(
			origin: OriginFor<T>,
			new: Perbill,
		) -> DispatchResultWithPostInfo {
			frame_system::ensure_root(origin)?;
			let old = <SequencerCommission<T>>::get();
			ensure!(old != new, Error::<T>::NoWritingSameValue);
			<SequencerCommission<T>>::put(new);
			Self::deposit_event(Event::SequencerCommissionSet { old, new });
			Ok(().into())
		}

		/// Set blocks per round
		/// - if called with `new` less than length of current round, will transition immediately
		/// in the next block
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::set_blocks_per_round())]
		pub fn set_blocks_per_round(origin: OriginFor<T>, new: u32) -> DispatchResultWithPostInfo {
			frame_system::ensure_root(origin)?;
			ensure!(new >= T::MinBlocksPerRound::get(), Error::<T>::CannotSetBelowMin);

			let mut round = <Round<T>>::get();
			let (now, first, old) = (round.current, round.first, round.length);

			ensure!(old != new, Error::<T>::NoWritingSameValue);
			ensure!(
				new > T::SequencerGroup::total_selected(),
				Error::<T>::RoundLengthMustBeGreaterThanTotalSelectedSequencers,
			);

			round.length = new;
			<Round<T>>::put(round);

			Self::deposit_event(Event::BlocksPerRoundSet {
				current_round: now,
				first_block: first,
				old,
				new,
			});

			Ok(().into())
		}

		/// Join the set of sequencer candidates
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::join_candidates(*candidate_count))]
		pub fn join_candidates(
			origin: OriginFor<T>,
			candidate_count: u32,
		) -> DispatchResultWithPostInfo {
			let acc = ensure_signed(origin)?;

			Self::join_candidates_inner(acc, candidate_count)
		}

		/// Request to leave the set of candidates. If successful, the account is immediately
		/// removed from the candidate pool to prevent selection as a sequencer.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_leave_candidates(*candidate_count))]
		pub fn schedule_leave_candidates(
			origin: OriginFor<T>,
			candidate_count: u32,
		) -> DispatchResultWithPostInfo {
			let sequencer = ensure_signed(origin)?;
			let mut state = <CandidateInfo<T>>::get(&sequencer).ok_or(Error::<T>::CandidateDNE)?;
			let (now, when) = state.schedule_leave::<T>()?;
			let mut candidates = <CandidatePool<T>>::get();
			ensure!(
				candidate_count >= candidates.0.len() as u32,
				Error::<T>::TooLowCandidateCountToLeaveCandidates
			);
			if candidates.remove(&Bond::from_owner(sequencer.clone())) {
				<CandidatePool<T>>::put(candidates);
			}
			<CandidateInfo<T>>::insert(&sequencer, state);
			Self::deposit_event(Event::CandidateScheduledExit {
				exit_allowed_round: now,
				candidate: sequencer,
				scheduled_exit: when,
			});
			Ok(().into())
		}

		/// Execute leave candidates request
		#[pallet::call_index(6)]
		#[pallet::weight(
			<T as Config>::WeightInfo::execute_leave_candidates_worst_case(*candidate_delegation_count)
		)]
		pub fn execute_leave_candidates(
			origin: OriginFor<T>,
			candidate: AccountIdOf<T>,
			candidate_delegation_count: u32,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			let state = <CandidateInfo<T>>::get(&candidate).ok_or(Error::<T>::CandidateDNE)?;
			ensure!(
				state.delegation_count <= candidate_delegation_count,
				Error::<T>::TooLowCandidateDelegationCountToLeaveCandidates
			);
			<Pallet<T>>::execute_leave_candidates_inner(candidate)
		}

		/// Cancel open request to leave candidates
		/// - only callable by sequencer account
		/// - result upon successful call is the candidate is active in the candidate pool
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_leave_candidates(*candidate_count))]
		pub fn cancel_leave_candidates(
			origin: OriginFor<T>,
			candidate_count: u32,
		) -> DispatchResultWithPostInfo {
			let sequencer = ensure_signed(origin)?;
			let mut state = <CandidateInfo<T>>::get(&sequencer).ok_or(Error::<T>::CandidateDNE)?;
			ensure!(state.is_leaving(), Error::<T>::CandidateNotLeaving);
			state.go_online();
			let mut candidates = <CandidatePool<T>>::get();
			ensure!(
				candidates.0.len() as u32 <= candidate_count,
				Error::<T>::TooLowCandidateCountWeightHintCancelLeaveCandidates
			);
			let maybe_inserted_candidate = candidates
				.try_insert(Bond { owner: sequencer.clone(), amount: state.total_counted })
				.map_err(|_| Error::<T>::CandidateLimitReached)?;
			ensure!(maybe_inserted_candidate, Error::<T>::AlreadyActive);
			<CandidatePool<T>>::put(candidates);
			<CandidateInfo<T>>::insert(&sequencer, state);
			Self::deposit_event(Event::CancelledCandidateExit { candidate: sequencer });
			Ok(().into())
		}

		/// Temporarily leave the set of sequencer candidates without unbonding
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::go_offline(T::MaxCandidates::get()))]
		pub fn go_offline(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let sequencer = ensure_signed(origin)?;
			<Pallet<T>>::go_offline_inner(sequencer)
		}

		/// Rejoin the set of sequencer candidates if previously had called `go_offline`
		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::go_online(T::MaxCandidates::get()))]
		pub fn go_online(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let sequencer = ensure_signed(origin)?;
			<Pallet<T>>::go_online_inner(sequencer)
		}

		/// Increase sequencer candidate self bond by `more`
		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::candidate_bond_more(T::MaxCandidates::get()))]
		pub fn candidate_bond_more(
			origin: OriginFor<T>,
			more: AssetBalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let candidate = ensure_signed(origin)?;
			<Pallet<T>>::candidate_bond_more_inner(candidate, more)
		}

		/// Request by sequencer candidate to decrease self bond by `less`
		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_candidate_bond_less())]
		pub fn schedule_candidate_bond_less(
			origin: OriginFor<T>,
			less: AssetBalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let sequencer = ensure_signed(origin)?;
			let mut state = <CandidateInfo<T>>::get(&sequencer).ok_or(Error::<T>::CandidateDNE)?;
			let when = state.schedule_bond_less::<T>(less)?;
			<CandidateInfo<T>>::insert(&sequencer, state);
			Self::deposit_event(Event::CandidateBondLessRequested {
				candidate: sequencer,
				amount_to_decrease: less,
				execute_round: when,
			});
			Ok(().into())
		}

		/// Execute pending request to adjust the sequencer candidate self bond
		#[pallet::call_index(12)]
		#[pallet::weight(<T as Config>::WeightInfo::execute_candidate_bond_less(T::MaxCandidates::get()))]
		pub fn execute_candidate_bond_less(
			origin: OriginFor<T>,
			candidate: AccountIdOf<T>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?; // we may want to reward this if caller != candidate
			<Pallet<T>>::execute_candidate_bond_less_inner(candidate)
		}

		/// Cancel pending request to adjust the sequencer candidate self bond
		#[pallet::call_index(13)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_candidate_bond_less())]
		pub fn cancel_candidate_bond_less(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let sequencer = ensure_signed(origin)?;
			let mut state = <CandidateInfo<T>>::get(&sequencer).ok_or(Error::<T>::CandidateDNE)?;
			state.cancel_bond_less::<T>(sequencer.clone())?;
			<CandidateInfo<T>>::insert(&sequencer, state);
			Ok(().into())
		}

		/// If caller is not a delegator and not a sequencer, then join the set of delegators
		/// If caller is a delegator, then makes delegation to change their delegation state
		#[pallet::call_index(14)]
		#[pallet::weight(
			<T as Config>::WeightInfo::delegate(
				*candidate_delegation_count,
				*delegation_count,
			)
		)]
		pub fn delegate(
			origin: OriginFor<T>,
			candidate: AccountIdOf<T>,
			amount: AssetBalanceOf<T>,
			candidate_delegation_count: u32,
			delegation_count: u32,
		) -> DispatchResultWithPostInfo {
			let delegator = ensure_signed(origin)?;
			Self::delegate_inner(
				candidate,
				delegator,
				amount,
				candidate_delegation_count,
				delegation_count,
			)
		}

		/// Request to revoke an existing delegation. If successful, the delegation is scheduled
		/// to be allowed to be revoked via the `execute_delegation_request` extrinsic.
		/// The delegation receives no rewards for the rounds while a revoke is pending.
		/// A revoke may not be performed if any other scheduled request is pending.
		#[pallet::call_index(15)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_revoke_delegation(
			T::MaxTopDelegationsPerCandidate::get() + T::MaxBottomDelegationsPerCandidate::get()
		))]
		pub fn schedule_revoke_delegation(
			origin: OriginFor<T>,
			sequencer: AccountIdOf<T>,
		) -> DispatchResultWithPostInfo {
			let delegator = ensure_signed(origin)?;
			Self::delegation_schedule_revoke(sequencer, delegator)
		}

		/// Bond more for delegators wrt a specific sequencer candidate.
		#[pallet::call_index(16)]
		#[pallet::weight(<T as Config>::WeightInfo::delegator_bond_more(
			T::MaxTopDelegationsPerCandidate::get() + T::MaxBottomDelegationsPerCandidate::get()
		))]
		pub fn delegator_bond_more(
			origin: OriginFor<T>,
			candidate: AccountIdOf<T>,
			more: AssetBalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let delegator = ensure_signed(origin)?;
			let (in_top, weight) = Self::delegation_bond_more_without_event(
				delegator.clone(),
				candidate.clone(),
				more.clone(),
			)?;
			Pallet::<T>::deposit_event(Event::DelegationIncreased {
				delegator,
				candidate,
				amount: more,
				in_top,
			});

			Ok(Some(weight).into())
		}

		/// Request bond less for delegators wrt a specific sequencer candidate. The delegation's
		/// rewards for rounds while the request is pending use the reduced bonded amount.
		/// A bond less may not be performed if any other scheduled request is pending.
		#[pallet::call_index(17)]
		#[pallet::weight(<T as Config>::WeightInfo::schedule_delegator_bond_less(
			T::MaxTopDelegationsPerCandidate::get() + T::MaxBottomDelegationsPerCandidate::get()
		))]
		pub fn schedule_delegator_bond_less(
			origin: OriginFor<T>,
			candidate: AccountIdOf<T>,
			less: AssetBalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let delegator = ensure_signed(origin)?;
			Self::delegation_schedule_bond_decrease(candidate, delegator, less)
		}

		/// Execute pending request to change an existing delegation
		#[pallet::call_index(18)]
		#[pallet::weight(<T as Config>::WeightInfo::execute_delegator_revoke_delegation_worst())]
		pub fn execute_delegation_request(
			origin: OriginFor<T>,
			delegator: AccountIdOf<T>,
			candidate: AccountIdOf<T>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?; // we may want to reward caller if caller != delegator
			Self::delegation_execute_scheduled_request(candidate, delegator)
		}

		/// Cancel request to change an existing delegation.
		#[pallet::call_index(19)]
		#[pallet::weight(<T as Config>::WeightInfo::cancel_delegation_request(350))]
		pub fn cancel_delegation_request(
			origin: OriginFor<T>,
			candidate: AccountIdOf<T>,
		) -> DispatchResultWithPostInfo {
			let delegator = ensure_signed(origin)?;
			Self::delegation_cancel_request(candidate, delegator)
		}

		/// Hotfix to remove existing empty entries for candidates that have left.
		#[pallet::call_index(20)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(2 * candidates.len() as u64, candidates.len() as u64)
		)]
		pub fn hotfix_remove_delegation_requests_exited_candidates(
			origin: OriginFor<T>,
			candidates: Vec<AccountIdOf<T>>,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(candidates.len() < 100, <Error<T>>::InsufficientBalance);
			for candidate in &candidates {
				ensure!(
					<CandidateInfo<T>>::get(&candidate).is_none(),
					<Error<T>>::CandidateNotLeaving
				);
				ensure!(
					<DelegationScheduledRequests<T>>::get(&candidate).is_empty(),
					<Error<T>>::CandidateNotLeaving
				);
			}

			for candidate in candidates {
				<DelegationScheduledRequests<T>>::remove(candidate);
			}

			Ok(().into())
		}

		/// Notify a sequencer is inactive during MaxOfflineRounds
		#[pallet::call_index(21)]
		#[pallet::weight(<T as Config>::WeightInfo::notify_inactive_sequencer())]
		pub fn notify_inactive_sequencer(
			origin: OriginFor<T>,
			sequencer: AccountIdOf<T>,
		) -> DispatchResult {
			ensure!(<EnableMarkingOffline<T>>::get(), <Error<T>>::MarkingOfflineNotEnabled);
			ensure_signed(origin)?;

			let mut sequencers_len = 0usize;
			let max_sequencers = T::SequencerGroup::total_selected();

			if let Some(len) = <SelectedCandidates<T>>::decode_len() {
				sequencers_len = len;
			};

			// Check sequencers length is not below or eq to 66% of max_sequencers.
			// We use saturating logic here with (2/3)
			// as it is dangerous to use floating point numbers directly.
			ensure!(
				sequencers_len * 3 > (max_sequencers * 2) as usize,
				<Error<T>>::TooLowSequencerCountToNotifyAsInactive
			);

			let round_info = <Round<T>>::get();
			let max_offline_rounds = T::MaxOfflineRounds::get();

			ensure!(round_info.current > max_offline_rounds, <Error<T>>::CurrentRoundTooLow);

			// Have rounds_to_check = [8,9]
			// in case we are in round 10 for instance
			// with MaxOfflineRounds = 2
			let first_round_to_check = round_info.current.saturating_sub(max_offline_rounds);
			let rounds_to_check = first_round_to_check..round_info.current;

			// If this counter is eq to max_offline_rounds,
			// the sequencer should be notified as inactive
			let mut inactive_counter: RoundIndex = 0u32;

			// Iter rounds to check
			//
			// - The sequencer has AtStake associated and their AwardedPts are zero
			//
			// If the previous condition is met in all rounds of rounds_to_check,
			// the sequencer is notified as inactive
			for r in rounds_to_check {
				let stake = <AtStake<T>>::get(r, &sequencer);
				let pts = <AwardedPts<T>>::get(r, &sequencer);

				if stake.is_some() && pts.is_zero() {
					inactive_counter = inactive_counter.saturating_add(1);
				}
			}

			if inactive_counter == max_offline_rounds {
				let _ = T::OnInactiveSequencer::on_inactive_sequencer(
					sequencer.clone(),
					round_info.current.saturating_sub(1),
				);
			} else {
				return Err(<Error<T>>::CannotBeNotifiedAsInactive.into());
			}

			Ok(().into())
		}

		/// Enable/Disable marking offline feature
		#[pallet::call_index(22)]
		#[pallet::weight(
			Weight::from_parts(3_000_000u64, 4_000u64)
				.saturating_add(T::DbWeight::get().writes(1u64))
		)]
		pub fn enable_marking_offline(origin: OriginFor<T>, value: bool) -> DispatchResult {
			ensure_root(origin)?;
			<EnableMarkingOffline<T>>::set(value);
			Ok(())
		}

		/// Force join the set of sequencer candidates.
		/// It will skip the minimum required bond check.
		#[pallet::call_index(23)]
		#[pallet::weight(<T as Config>::WeightInfo::join_candidates(*candidate_count))]
		pub fn force_join_candidates(
			origin: OriginFor<T>,
			account: AccountIdOf<T>,
			_bond: AssetBalanceOf<T>,
			candidate_count: u32,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::join_candidates_inner(account, candidate_count)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn account_id() -> AccountIdOf<T> {
			T::PalletAccount::get().into_account_truncating()
		}

		pub(crate) fn delegate_inner(
			candidate: AccountIdOf<T>,
			delegator: AccountIdOf<T>,
			amount: AssetBalanceOf<T>,
			candidate_delegation_count_hint: u32,
			delegation_count_hint: u32,
		) -> DispatchResultWithPostInfo {
			// ensure that the candidate is active
			ensure!(
				<CandidateInfo<T>>::get(&candidate).map_or(false, |c| c.is_active()),
				Error::<T>::CandidateDNE
			);

			// check that caller can transfer the amount before any changes to storage
			ensure!(
				T::Assets::reducible_balance(
					T::BTC::get(),
					&delegator,
					Preservation::Expendable,
					Fortitude::Polite
				) >= amount,
				Error::<T>::InsufficientBalance
			);
			ensure!(amount >= T::MinDelegation::get(), Error::<T>::DelegationBelowMin);

			let delegator_state = if let Some(mut state) = <DelegatorState<T>>::get(&delegator) {
				// delegation after first
				ensure!(
					delegation_count_hint >= state.delegations.0.len() as u32,
					Error::<T>::TooLowDelegationCountToDelegate
				);
				ensure!(
					(state.delegations.0.len() as u32) < T::MaxDelegationsPerDelegator::get(),
					Error::<T>::ExceedMaxDelegationsPerDelegator
				);
				ensure!(
					state.add_delegation(Bond { owner: candidate.clone(), amount }),
					Error::<T>::AlreadyDelegatedCandidate
				);
				state
			} else {
				// first delegation
				ensure!(!<Pallet<T>>::is_candidate(&delegator), Error::<T>::CandidateExists);
				Delegator::new(delegator.clone(), candidate.clone(), amount)
			};
			let mut candidate_state =
				<CandidateInfo<T>>::get(&candidate).ok_or(Error::<T>::CandidateDNE)?;
			ensure!(
				candidate_delegation_count_hint >= candidate_state.delegation_count,
				Error::<T>::TooLowCandidateDelegationCountToDelegate
			);

			// add delegation to candidate
			let (delegator_position, less_total_staked) = candidate_state
				.add_delegation::<T>(&candidate, Bond { owner: delegator.clone(), amount })?;

			// lock delegator amount
			// 需要变成转账模式，因为assets模块没有实现freeze的功能
			T::Assets::transfer(
				T::BTC::get(),
				&delegator,
				&Self::account_id(),
				amount,
				Preservation::Expendable,
			)
			.map_err(|_| Error::<T>::TransferFailed)?;

			// adjust total locked,
			// only is_some if kicked the lowest bottom as a consequence of this new delegation
			let net_total_increase = if let Some(less) = less_total_staked {
				amount.saturating_sub(less)
			} else {
				amount
			};
			let new_total_locked = <Total<T>>::get().saturating_add(net_total_increase);

			<Total<T>>::put(new_total_locked);
			<CandidateInfo<T>>::insert(&candidate, candidate_state);
			<DelegatorState<T>>::insert(&delegator, delegator_state);
			<Pallet<T>>::deposit_event(Event::Delegation {
				delegator,
				locked_amount: amount,
				candidate,
				delegator_position,
			});

			Ok(().into())
		}

		pub fn set_candidate_bond_to_zero(acc: &AccountIdOf<T>) -> Weight {
			let actual_weight =
				<T as Config>::WeightInfo::set_candidate_bond_to_zero(T::MaxCandidates::get());
			if let Some(mut state) = <CandidateInfo<T>>::get(&acc) {
				let _ = state.bond_less::<T>(acc.clone(), state.bond);
				<CandidateInfo<T>>::insert(&acc, state);
			}
			actual_weight
		}

		pub fn is_delegator(acc: &AccountIdOf<T>) -> bool {
			<DelegatorState<T>>::get(acc).is_some()
		}

		pub fn is_candidate(acc: &AccountIdOf<T>) -> bool {
			<CandidateInfo<T>>::get(acc).is_some()
		}

		pub fn join_candidates_inner(
			acc: AccountIdOf<T>,
			candidate_count: u32,
		) -> DispatchResultWithPostInfo {
			ensure!(!Self::is_candidate(&acc), Error::<T>::CandidateExists);
			ensure!(!Self::is_delegator(&acc), Error::<T>::DelegatorExists);
			let mut candidates = <CandidatePool<T>>::get();
			let old_count = candidates.0.len() as u32;
			ensure!(
				candidate_count >= old_count,
				Error::<T>::TooLowCandidateCountWeightHintJoinCandidates
			);
			let maybe_inserted_candidate = candidates
				.try_insert(Bond { owner: acc.clone(), amount: Zero::zero() })
				.map_err(|_| Error::<T>::CandidateLimitReached)?;
			ensure!(maybe_inserted_candidate, Error::<T>::CandidateExists);

			let lock_amount = T::MinCandidateStk::get();
			ensure!(
				T::Currency::free_balance(&acc) >= lock_amount,
				Error::<T>::InsufficientBalance,
			);
			T::Currency::set_lock(SEQUENCER_LOCK_ID, &acc, lock_amount, WithdrawReasons::all());

			let candidate = CandidateMetadata::new(Zero::zero());
			<CandidateInfo<T>>::insert(&acc, candidate);
			let empty_delegations: Delegations<AccountIdOf<T>, AssetBalanceOf<T>> =
				Default::default();
			// insert empty top delegations
			<TopDelegations<T>>::insert(&acc, empty_delegations.clone());
			// insert empty bottom delegations
			<BottomDelegations<T>>::insert(&acc, empty_delegations);
			<CandidatePool<T>>::put(candidates);

			Self::deposit_event(Event::JoinedSequencerCandidates {
				account: acc,
				amount_locked: lock_amount,
			});
			Ok(().into())
		}

		pub fn go_offline_inner(sequencer: AccountIdOf<T>) -> DispatchResultWithPostInfo {
			let mut state = <CandidateInfo<T>>::get(&sequencer).ok_or(Error::<T>::CandidateDNE)?;
			let mut candidates = <CandidatePool<T>>::get();
			let actual_weight = <T as Config>::WeightInfo::go_offline(candidates.0.len() as u32);

			ensure!(
				state.is_active(),
				DispatchErrorWithPostInfo {
					post_info: Some(actual_weight).into(),
					error: <Error<T>>::AlreadyOffline.into(),
				}
			);
			state.go_offline();

			if candidates.remove(&Bond::from_owner(sequencer.clone())) {
				<CandidatePool<T>>::put(candidates);
			}
			<CandidateInfo<T>>::insert(&sequencer, state);
			Self::deposit_event(Event::CandidateWentOffline { candidate: sequencer });
			Ok(Some(actual_weight).into())
		}

		pub fn go_online_inner(sequencer: AccountIdOf<T>) -> DispatchResultWithPostInfo {
			let mut state = <CandidateInfo<T>>::get(&sequencer).ok_or(Error::<T>::CandidateDNE)?;
			let mut candidates = <CandidatePool<T>>::get();
			let actual_weight = <T as Config>::WeightInfo::go_online(candidates.0.len() as u32);

			ensure!(
				!state.is_active(),
				DispatchErrorWithPostInfo {
					post_info: Some(actual_weight).into(),
					error: <Error<T>>::AlreadyActive.into(),
				}
			);
			ensure!(
				!state.is_leaving(),
				DispatchErrorWithPostInfo {
					post_info: Some(actual_weight).into(),
					error: <Error<T>>::CannotGoOnlineIfLeaving.into(),
				}
			);
			state.go_online();

			let maybe_inserted_candidate = candidates
				.try_insert(Bond { owner: sequencer.clone(), amount: state.total_counted })
				.map_err(|_| Error::<T>::CandidateLimitReached)?;
			ensure!(
				maybe_inserted_candidate,
				DispatchErrorWithPostInfo {
					post_info: Some(actual_weight).into(),
					error: <Error<T>>::AlreadyActive.into(),
				},
			);

			<CandidatePool<T>>::put(candidates);
			<CandidateInfo<T>>::insert(&sequencer, state);
			Self::deposit_event(Event::CandidateBackOnline { candidate: sequencer });
			Ok(Some(actual_weight).into())
		}

		pub fn candidate_bond_more_inner(
			sequencer: AccountIdOf<T>,
			more: AssetBalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let mut state = <CandidateInfo<T>>::get(&sequencer).ok_or(Error::<T>::CandidateDNE)?;
			let actual_weight =
				<T as Config>::WeightInfo::candidate_bond_more(T::MaxCandidates::get());

			state.bond_more::<T>(sequencer.clone(), more).map_err(|err| {
				DispatchErrorWithPostInfo { post_info: Some(actual_weight).into(), error: err }
			})?;
			let (is_active, total_counted) = (state.is_active(), state.total_counted);
			<CandidateInfo<T>>::insert(&sequencer, state);
			if is_active {
				Self::update_active(sequencer, total_counted);
			}
			Ok(Some(actual_weight).into())
		}

		pub fn execute_candidate_bond_less_inner(
			candidate: AccountIdOf<T>,
		) -> DispatchResultWithPostInfo {
			let mut state = <CandidateInfo<T>>::get(&candidate).ok_or(Error::<T>::CandidateDNE)?;
			let actual_weight =
				<T as Config>::WeightInfo::execute_candidate_bond_less(T::MaxCandidates::get());

			state.execute_bond_less::<T>(candidate.clone()).map_err(|err| {
				DispatchErrorWithPostInfo { post_info: Some(actual_weight).into(), error: err }
			})?;
			<CandidateInfo<T>>::insert(&candidate, state);
			Ok(Some(actual_weight).into())
		}

		pub fn execute_leave_candidates_inner(
			candidate: AccountIdOf<T>,
		) -> DispatchResultWithPostInfo {
			let state = <CandidateInfo<T>>::get(&candidate).ok_or(Error::<T>::CandidateDNE)?;
			// TODO use these to return actual weight used via `execute_leave_candidates_worst_case`
			let actual_delegation_count = state.delegation_count;
			let actual_weight = <T as Config>::WeightInfo::execute_leave_candidates_worst_case(
				actual_delegation_count,
			);

			state.can_leave::<T>().map_err(|err| DispatchErrorWithPostInfo {
				post_info: Some(actual_weight).into(),
				error: err,
			})?;
			let return_stake = |bond: Bond<AccountIdOf<T>, AssetBalanceOf<T>>| {
				// remove delegation from delegator state
				let mut delegator = DelegatorState::<T>::get(&bond.owner).expect(
					"Sequencer state and delegator state are consistent. 
						Sequencer state has a record of this delegation. Therefore, 
						Delegator state also has a record. qed.",
				);

				if let Some(remaining) = delegator.rm_delegation::<T>(&candidate) {
					Self::delegation_remove_request_with_state(
						&candidate,
						&bond.owner,
						&mut delegator,
					);

					if remaining.is_zero() {
						// we do not remove the scheduled delegation requests from other sequencers
						// since it is assumed that they were removed incrementally before only the
						// last delegation was left.
						<DelegatorState<T>>::remove(&bond.owner);
					} else {
						<DelegatorState<T>>::insert(&bond.owner, delegator);
					}
				}
			};
			// total backing stake is at least the candidate self bond
			let mut total_backing = state.bond;
			// return all top delegations
			let top_delegations =
				<TopDelegations<T>>::take(&candidate).expect("CandidateInfo existence checked");
			for bond in top_delegations.delegations {
				return_stake(bond);
			}
			total_backing = total_backing.saturating_add(top_delegations.total);
			// return all bottom delegations
			let bottom_delegations =
				<BottomDelegations<T>>::take(&candidate).expect("CandidateInfo existence checked");
			for bond in bottom_delegations.delegations {
				return_stake(bond);
			}
			total_backing = total_backing.saturating_add(bottom_delegations.total);

			// return join deposit to sequencer
			T::Currency::remove_lock(SEQUENCER_LOCK_ID, &candidate);

			<CandidateInfo<T>>::remove(&candidate);
			<DelegationScheduledRequests<T>>::remove(&candidate);

			<TopDelegations<T>>::remove(&candidate);
			<BottomDelegations<T>>::remove(&candidate);
			let new_total_staked = <Total<T>>::get().saturating_sub(total_backing);
			<Total<T>>::put(new_total_staked);
			Self::deposit_event(Event::CandidateLeft {
				ex_candidate: candidate,
				unlocked_amount: total_backing,
				new_total_amt_locked: new_total_staked,
			});
			Ok(Some(actual_weight).into())
		}

		/// Caller must ensure candidate is active before calling
		pub(crate) fn update_active(candidate: AccountIdOf<T>, total: AssetBalanceOf<T>) {
			let mut candidates = <CandidatePool<T>>::get();
			candidates.remove(&Bond::from_owner(candidate.clone()));
			candidates.try_insert(Bond { owner: candidate, amount: total }).expect(
				"the candidate is removed in previous step so the length cannot increase; qed",
			);
			<CandidatePool<T>>::put(candidates);
		}

		/// Remove delegation from candidate state
		/// Amount input should be retrieved from delegator and it informs the storage lookups
		pub(crate) fn delegator_leaves_candidate(
			candidate: AccountIdOf<T>,
			delegator: AccountIdOf<T>,
			amount: AssetBalanceOf<T>,
		) -> DispatchResult {
			let mut state = <CandidateInfo<T>>::get(&candidate).ok_or(Error::<T>::CandidateDNE)?;
			state.rm_delegation_if_exists::<T>(&candidate, delegator.clone(), amount)?;
			let new_total_locked = <Total<T>>::get().saturating_sub(amount);
			<Total<T>>::put(new_total_locked);
			let new_total = state.total_counted;
			<CandidateInfo<T>>::insert(&candidate, state);
			Self::deposit_event(Event::DelegatorLeftCandidate {
				delegator,
				candidate,
				unstaked_amount: amount,
				total_candidate_staked: new_total,
			});
			Ok(())
		}

		pub(crate) fn prepare_staking_payouts(round_info: RoundInfo<BlockNumberFor<T>>) -> Weight {
			let RoundInfo { current: now, .. } = round_info;

			// This function is called right after the round index increment,
			// and the goal is to compute the payout informations for the round that just ended.
			// We don't need to saturate here because the genesis round is 1.
			let prepare_payout_for_round = now - 1;

			// Return early if there is no blocks for this round
			if <Points<T>>::get(prepare_payout_for_round).is_zero() {
				return Weight::zero();
			}

			// Get the reward issuance for the round
			let total_issuance = T::RoundReward::get();

			let payout = DelayedPayout {
				total_staking_reward: total_issuance,
				sequencer_commission: <SequencerCommission<T>>::get(),
			};

			<DelayedPayouts<T>>::insert(prepare_payout_for_round, payout);

			<T as Config>::WeightInfo::prepare_staking_payouts()
		}

		/// Wrapper around pay_one_sequencer_reward which handles the following logic:
		/// * whether or not a payout needs to be made
		/// * cleaning up when payouts are done
		/// * returns the weight consumed by pay_one_sequencer_reward if applicable
		fn handle_delayed_payouts(now: RoundIndex) -> Weight {
			let delay = T::RewardPaymentDelay::get();

			// don't underflow uint
			if now < delay {
				return Weight::from_parts(0u64, 0);
			}

			let paid_for_round = now.saturating_sub(delay);

			if let Some(payout_info) = <DelayedPayouts<T>>::get(paid_for_round) {
				let result = Self::pay_one_sequencer_reward(paid_for_round, payout_info);

				// clean up storage items that we no longer need
				if matches!(result.0, RewardPayment::Finished) {
					<DelayedPayouts<T>>::remove(paid_for_round);
					<Points<T>>::remove(paid_for_round);
				}
				result.1 // weight consumed by pay_one_sequencer_reward
			} else {
				Weight::from_parts(0u64, 0)
			}
		}

		/// Payout a single sequencer from the given round.
		///
		/// Returns an optional tuple of (Sequencer's AccountId, total paid)
		/// or None if there were no more payouts to be made for the round.
		pub(crate) fn pay_one_sequencer_reward(
			paid_for_round: RoundIndex,
			payout_info: DelayedPayout<BalanceOf<T>>,
		) -> (RewardPayment, Weight) {
			// 'early_weight' tracks weight used for reads/writes done early in this fn before its
			// early-exit codepaths.
			let mut early_weight = Weight::zero();

			// TODO: it would probably be optimal to roll Points into the DelayedPayouts storage
			// item so that we do fewer reads each block
			let total_points = <Points<T>>::get(paid_for_round);
			early_weight = early_weight.saturating_add(T::DbWeight::get().reads_writes(1, 0));

			if total_points.is_zero() {
				// TODO: this case is obnoxious... it's a value query, so it could mean one of two
				// different logic errors:
				// 1. we removed it before we should have
				// 2. we called pay_one_sequencer_reward when we were actually done with deferred
				//    payouts
				log::warn!("pay_one_sequencer_reward called with no <Points<T>> for the round!");
				return (RewardPayment::Finished, early_weight);
			}

			let sequencer_fee = payout_info.sequencer_commission;
			let sequencer_issuance = sequencer_fee * payout_info.total_staking_reward;
			let staking_issuance =
				payout_info.total_staking_reward.saturating_sub(sequencer_issuance);

			if let Some((sequencer, state)) =
				<AtStake<T>>::iter_prefix(paid_for_round).drain().next()
			{
				// read and kill AtStake
				early_weight = early_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

				// Take the awarded points for the sequencer
				let pts = <AwardedPts<T>>::take(paid_for_round, &sequencer);
				// read and kill AwardedPts
				early_weight = early_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
				if pts == 0 {
					return (RewardPayment::Skipped, early_weight);
				}

				// 'extra_weight' tracks weight returned from fns that we delegate to which can't be
				// known ahead of time.
				let mut extra_weight = Weight::zero();
				let pct_due = Perbill::from_rational(pts, total_points);
				let total_paid = pct_due * staking_issuance;
				let mut amt_due = total_paid;

				let num_delegators = state.delegations.len();
				let mut num_paid_delegations = 0u32;

				let num_scheduled_requests =
					<DelegationScheduledRequests<T>>::decode_len(&sequencer).unwrap_or_default();
				if state.delegations.is_empty() {
					// solo sequencer with no delegators
					extra_weight = extra_weight
						.saturating_add(T::PayoutSequencerReward::payout_sequencer_reward(
							paid_for_round,
							sequencer.clone(),
							amt_due,
						))
						.saturating_add(T::OnSequencerPayout::on_sequencer_payout(
							paid_for_round,
							sequencer.clone(),
							amt_due,
						));
				} else {
					// pay sequencer first; commission + due_portion
					let sequencer_pct = Perbill::from_rational(state.bond, state.total);
					let commission = pct_due * sequencer_issuance;
					amt_due = amt_due.saturating_sub(commission);
					let sequencer_reward = (sequencer_pct * amt_due).saturating_add(commission);
					extra_weight = extra_weight
						.saturating_add(T::PayoutSequencerReward::payout_sequencer_reward(
							paid_for_round,
							sequencer.clone(),
							sequencer_reward,
						))
						.saturating_add(T::OnSequencerPayout::on_sequencer_payout(
							paid_for_round,
							sequencer.clone(),
							sequencer_reward,
						));

					// pay delegators due portion
					for Bond { owner, amount } in state.delegations {
						let percent = Perbill::from_rational(amount, state.total);
						let due = percent * amt_due;
						if !due.is_zero() {
							num_paid_delegations += 1u32;
							Self::payout_reward(due, owner.clone());
						}
					}
				}

				extra_weight = extra_weight.saturating_add(
					<T as Config>::WeightInfo::pay_one_sequencer_reward_best(
						num_paid_delegations,
						num_scheduled_requests as u32,
					),
				);

				(
					RewardPayment::Paid,
					<T as Config>::WeightInfo::pay_one_sequencer_reward(num_delegators as u32)
						.saturating_add(extra_weight),
				)
			} else {
				// Note that we don't clean up storage here; it is cleaned up in
				// handle_delayed_payouts()
				(RewardPayment::Finished, Weight::from_parts(0u64, 0))
			}
		}

		/// Compute the top `TotalSelected` candidates in the CandidatePool and return
		/// a vec of their AccountIds (sorted by AccountId).
		///
		/// If the returned vec is empty, the previous candidates should be used.
		pub fn compute_top_candidates() -> Vec<AccountIdOf<T>> {
			let top_n = T::SequencerGroup::total_selected() as usize;
			if top_n == 0 {
				return vec![];
			}

			let candidates = <CandidatePool<T>>::get().0;

			// If the number of candidates is greater than top_n, select the candidates with higher
			// amount. Otherwise, return all the candidates.
			if candidates.len() > top_n {
				// Partially sort candidates such that element at index `top_n - 1` is sorted, and
				// all the elements in the range 0..top_n are the top n elements.
				let sorted_candidates = candidates
					.try_mutate(|inner| {
						inner.select_nth_unstable_by(top_n - 1, |a, b| {
							// Order by amount, then owner. The owner is needed to ensure a stable
							// order when two accounts have the same amount.
							a.amount.cmp(&b.amount).then_with(|| a.owner.cmp(&b.owner)).reverse()
						});
					})
					.expect("sort cannot increase item count; qed");

				let mut sequencers =
					sorted_candidates.into_iter().take(top_n).map(|x| x.owner).collect::<Vec<_>>();

				// Sort sequencers by AccountId
				sequencers.sort();

				sequencers
			} else {
				// Return all candidates
				// The candidates are already sorted by AccountId, so no need to sort again
				candidates.into_iter().map(|x| x.owner).collect::<Vec<_>>()
			}
		}
		/// Best as in most cumulatively supported in terms of stake
		/// Returns [sequencer_count, delegation_count, total staked]
		pub(crate) fn select_top_candidates(
			next: RoundIndex,
		) -> (Weight, u32, u32, AssetBalanceOf<T>) {
			let (mut sequencer_count, mut delegation_count, mut total) =
				(0u32, 0u32, AssetBalanceOf::<T>::zero());
			// choose the top TotalSelected qualified candidates, ordered by stake
			let sequencers = Self::compute_top_candidates();
			if sequencers.is_empty() {
				// SELECTION FAILED TO SELECT >=1 SEQUENCER => select sequencers from previous round
				let current_round = next.saturating_sub(1u32);
				let mut total_per_candidate: BTreeMap<AccountIdOf<T>, AssetBalanceOf<T>> =
					BTreeMap::new();
				// set next round AtStake to current round AtStake
				for (account, snapshot) in <AtStake<T>>::iter_prefix(current_round) {
					sequencer_count = sequencer_count.saturating_add(1u32);
					delegation_count =
						delegation_count.saturating_add(snapshot.delegations.len() as u32);
					total = total.saturating_add(snapshot.total);
					total_per_candidate.insert(account.clone(), snapshot.total);
					<AtStake<T>>::insert(next, account, snapshot);
				}
				// `SelectedCandidates` remains unchanged from last round
				// emit SequencerChosen event for tools that use this event
				for candidate in <SelectedCandidates<T>>::get() {
					let snapshot_total = total_per_candidate
						.get(&candidate)
						.expect("all selected candidates have snapshots");
					Self::deposit_event(Event::SequencerChosen {
						round: next,
						sequencer_account: candidate,
						total_exposed_amount: *snapshot_total,
					})
				}
				let weight = <T as Config>::WeightInfo::select_top_candidates(0, 0);
				return (weight, sequencer_count, delegation_count, total);
			}

			// snapshot exposure for round for weighting reward distribution
			for account in sequencers.iter() {
				let state = <CandidateInfo<T>>::get(account)
					.expect("all members of CandidateQ must be candidates");

				sequencer_count = sequencer_count.saturating_add(1u32);
				delegation_count = delegation_count.saturating_add(state.delegation_count);
				total = total.saturating_add(state.total_counted);
				let CountedDelegations { uncounted_stake, rewardable_delegations } =
					Self::get_rewardable_delegators(&account);
				let total_counted = state.total_counted.saturating_sub(uncounted_stake);

				let snapshot = SequencerSnapshot {
					bond: state.bond,
					delegations: rewardable_delegations,
					total: total_counted,
				};
				<AtStake<T>>::insert(next, account, snapshot);
				Self::deposit_event(Event::SequencerChosen {
					round: next,
					sequencer_account: account.clone(),
					total_exposed_amount: state.total_counted,
				});
			}
			// insert canonical sequencer set to the selected candidates storage for the next
			// round
			<SelectedCandidates<T>>::put(
				BoundedVec::try_from(sequencers)
					.expect("subset of sequencers is always less than or equal to max candidates"),
			);

			let avg_delegator_count = delegation_count.checked_div(sequencer_count).unwrap_or(0);
			let weight = <T as Config>::WeightInfo::select_top_candidates(
				sequencer_count,
				avg_delegator_count,
			);
			(weight, sequencer_count, delegation_count, total)
		}

		/// Apply the delegator intent for revoke and decrease in order to build the
		/// effective list of delegators with their intended bond amount.
		///
		/// This will:
		/// - if [DelegationChange::Revoke] is outstanding, set the bond amount to 0.
		/// - if [DelegationChange::Decrease] is outstanding, subtract the bond by specified amount.
		/// - else, do nothing
		///
		/// The intended bond amounts will be used while calculating rewards.
		pub(crate) fn get_rewardable_delegators(
			sequencer: &AccountIdOf<T>,
		) -> CountedDelegations<T> {
			let requests = <DelegationScheduledRequests<T>>::get(sequencer)
				.into_iter()
				.map(|x| (x.delegator, x.action))
				.collect::<BTreeMap<_, _>>();
			let mut uncounted_stake = AssetBalanceOf::<T>::zero();
			let rewardable_delegations = <TopDelegations<T>>::get(sequencer)
				.expect("all members of CandidateQ must be candidates")
				.delegations
				.into_iter()
				.map(|mut bond| {
					bond.amount = match requests.get(&bond.owner) {
						None => bond.amount,
						Some(DelegationAction::Revoke(_)) => {
							uncounted_stake = uncounted_stake.saturating_add(bond.amount);
							AssetBalanceOf::<T>::zero()
						},
						Some(DelegationAction::Decrease(amount)) => {
							uncounted_stake = uncounted_stake.saturating_add(*amount);
							bond.amount.saturating_sub(*amount)
						},
					};

					bond
				})
				.collect();
			CountedDelegations { uncounted_stake, rewardable_delegations }
		}

		/// This function exists as a helper to delegator_bond_more & auto_compound functionality.
		/// Any changes to this function must align with both user-initiated bond increases and
		/// auto-compounding bond increases.
		/// Any feature-specific preconditions should be validated before this function is invoked.
		/// Any feature-specific events must be emitted after this function is invoked.
		pub fn delegation_bond_more_without_event(
			delegator: AccountIdOf<T>,
			candidate: AccountIdOf<T>,
			more: AssetBalanceOf<T>,
		) -> Result<
			(bool, Weight),
			DispatchErrorWithPostInfo<frame_support::dispatch::PostDispatchInfo>,
		> {
			let mut state = <DelegatorState<T>>::get(&delegator).ok_or(Error::<T>::DelegatorDNE)?;
			ensure!(
				!Self::delegation_request_revoke_exists(&candidate, &delegator),
				Error::<T>::PendingDelegationRevoke
			);

			let actual_weight = <T as Config>::WeightInfo::delegator_bond_more(
				<DelegationScheduledRequests<T>>::decode_len(&candidate).unwrap_or_default() as u32,
			);
			let in_top =
				state.increase_delegation::<T>(candidate.clone(), more).map_err(|err| {
					DispatchErrorWithPostInfo { post_info: Some(actual_weight).into(), error: err }
				})?;

			Ok((in_top, actual_weight))
		}

		pub fn payout_reward(amt: BalanceOf<T>, delegator: AccountIdOf<T>) {
			let reward_account = Self::account_id();

			if T::Currency::transfer(
				&reward_account,
				&delegator,
				amt,
				ExistenceRequirement::AllowDeath,
			)
			.is_ok()
			{
				Self::deposit_event(Event::Rewarded { account: delegator.clone(), rewards: amt });
			};
		}
	}

	// Should be call by outer module to add points to sequencers
	impl<T: Config> SequencerStakingInterface<AccountIdOf<T>> for Pallet<T> {
		fn award_points_to_sequencer(
			sequencer: AccountIdOf<T>,
			points: RewardPoint,
		) -> DispatchResult {
			let now = <Round<T>>::get().current;
			let total_points = <AwardedPts<T>>::get(now, &sequencer).saturating_add(points);
			<AwardedPts<T>>::insert(now, sequencer, total_points);
			<Points<T>>::mutate(now, |x| *x = x.saturating_add(points));

			Ok(())
		}
	}
}
