use crate::{
	delegation_requests::{DelegationAction, ScheduledRequest},
	mock::{
		roll_blocks, roll_to, roll_to_round_begin, roll_to_round_end, Balances, BlockNumber,
		ExtBuilder, RuntimeOrigin, SequencerStaking, Test,
	},
	AtStake, Bond, DelegationScheduledRequests, EnableMarkingOffline, Error, SequencerStatus, *,
};
use frame_support::{
	assert_noop, assert_ok,
	pallet_prelude::*,
	traits::{
		fungibles::Inspect,
		tokens::{Fortitude, Preservation},
		Currency, WithdrawReasons,
	},
	BoundedVec,
};
use mock::*;
use pallet_sequencer_grouping::SequencerGroup;
use sp_runtime::{traits::Zero, DispatchError::Module, ModuleError, Perbill};

// ~~ ROOT ~~

#[test]
fn charge_reward_account_works() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20), (3, 20), (4, 20), (5, 20)])
		.with_candidates(vec![1, 2, 3, 4, 5])
		.build()
		.execute_with(|| {
			// Round 2
			roll_to_round_begin(2);

			let pallet_account = SequencerStaking::account_id();

			// get the balance of native token of the reward account
			assert_eq!(<Test as crate::Config>::Currency::free_balance(&1), 20);
			assert_eq!(<Test as crate::Config>::Currency::free_balance(&pallet_account), 0);

			// Call 'charge_reward_account' extrinsic
			assert_ok!(SequencerStaking::charge_reward_account(RuntimeOrigin::signed(1), 10));

			assert_eq!(<Test as crate::Config>::Currency::free_balance(&1), 10);
			assert_eq!(<Test as crate::Config>::Currency::free_balance(&pallet_account), 10);
		});
}

#[test]
fn invalid_root_origin_fails() {
	ExtBuilder::default().build().execute_with(|| {
		// assert_noop!(
		// 	SequencerStaking::set_total_selected(RuntimeOrigin::signed(45), 6u32),
		// 	sp_runtime::DispatchError::BadOrigin
		// );
		assert_noop!(
			SequencerStaking::set_sequencer_commission(
				RuntimeOrigin::signed(45),
				Perbill::from_percent(5)
			),
			sp_runtime::DispatchError::BadOrigin
		);
		assert_noop!(
			SequencerStaking::set_blocks_per_round(RuntimeOrigin::signed(45), 3u32),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

// SET TOTAL SELECTED

// #[test]
// fn set_total_selected_fails_if_above_blocks_per_round() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		assert_eq!(Round::<Test>::get().length, 5); // test relies on this
// 		assert_noop!(
// 			SequencerStaking::set_total_selected(RuntimeOrigin::root(), 6u32),
// 			Error::<Test>::RoundLengthMustBeGreaterThanTotalSelectedSequencers,
// 		);
// 	});
// }

// #[test]
// fn set_total_selected_fails_if_above_max_candidates() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		assert_eq!(<Test as crate::Config>::MaxCandidates::get(), 200); // test relies on this
// 		assert_noop!(
// 			SequencerStaking::set_total_selected(RuntimeOrigin::root(), 201u32),
// 			Error::<Test>::CannotSetAboveMaxCandidates,
// 		);
// 	});
// }

// #[test]
// fn set_total_selected_fails_if_equal_to_blocks_per_round() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		assert_ok!(SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 10u32));
// 		assert_noop!(
// 			SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 1u32),
// 			Error::<Test>::RoundLengthMustBeGreaterThanTotalSelectedSequencers,
// 		);
// 	});
// }

// #[test]
// fn set_total_selected_passes_if_below_blocks_per_round() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		assert_ok!(SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 10u32));
// 		assert_ok!(SequencerStaking::set_total_selected(RuntimeOrigin::root(), 9u32));
// 	});
// }

#[test]
fn set_blocks_per_round_fails_if_below_total_selected() {
	ExtBuilder::default().build().execute_with(|| {
		// assert_ok!(SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 20u32));
		// assert_ok!(SequencerStaking::set_total_selected(RuntimeOrigin::root(), 10u32));
		assert_ok!(<Test as crate::Config>::SequencerGroup::set_group_metric(
			RuntimeOrigin::root(),
			2u32,
			5u32
		));
		assert_noop!(
			SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 9u32),
			Error::<Test>::RoundLengthMustBeGreaterThanTotalSelectedSequencers,
		);
	});
}

#[test]
fn set_blocks_per_round_fails_if_equal_to_total_selected() {
	ExtBuilder::default().build().execute_with(|| {
		// assert_ok!(SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 10u32));
		// assert_ok!(SequencerStaking::set_total_selected(RuntimeOrigin::root(), 9u32));
		assert_ok!(<Test as crate::Config>::SequencerGroup::set_group_metric(
			RuntimeOrigin::root(),
			2u32,
			3u32
		));
		assert_noop!(
			SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 6u32),
			Error::<Test>::RoundLengthMustBeGreaterThanTotalSelectedSequencers,
		);
	});
}

#[test]
fn set_blocks_per_round_passes_if_above_total_selected() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Round::<Test>::get().length, 5); // test relies on this
		assert_ok!(<Test as crate::Config>::SequencerGroup::set_group_metric(
			RuntimeOrigin::root(),
			2u32,
			3u32
		));
		assert_ok!(SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 7u32));
	});
}

// #[test]
// fn set_total_selected_storage_updates_correctly() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		// round length must be >= total_selected, so update that first
// 		assert_ok!(SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 10u32));
//
// 		assert_eq!(TotalSelected::<Test>::get(), 5u32);
// 		assert_ok!(SequencerStaking::set_total_selected(RuntimeOrigin::root(), 6u32));
// 		assert_eq!(TotalSelected::<Test>::get(), 6u32);
// 	});
// }

// #[test]
// fn cannot_set_total_selected_to_current_total_selected() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		assert_noop!(
// 			SequencerStaking::set_total_selected(RuntimeOrigin::root(), 5u32),
// 			Error::<Test>::NoWritingSameValue
// 		);
// 	});
// }

// #[test]
// fn cannot_set_total_selected_below_module_min() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		assert_noop!(
// 			SequencerStaking::set_total_selected(RuntimeOrigin::root(), 4u32),
// 			Error::<Test>::CannotSetBelowMin
// 		);
// 	});
// }

// SET COLLATOR COMMISSION

#[test]
fn set_sequencer_commission_storage_updates_correctly() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SequencerCommission::<Test>::get(), Perbill::from_percent(20));
		assert_ok!(SequencerStaking::set_sequencer_commission(
			RuntimeOrigin::root(),
			Perbill::from_percent(5)
		));
		assert_eq!(SequencerCommission::<Test>::get(), Perbill::from_percent(5));
	});
}

#[test]
fn cannot_set_sequencer_commission_to_current_sequencer_commission() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			SequencerStaking::set_sequencer_commission(
				RuntimeOrigin::root(),
				Perbill::from_percent(20)
			),
			Error::<Test>::NoWritingSameValue
		);
	});
}

// SET BLOCKS PER ROUND

#[test]
fn set_blocks_per_round_storage_updates_correctly() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Round::<Test>::get().length, 5);
		assert_ok!(SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 6u32));
		assert_eq!(Round::<Test>::get().length, 6);
	});
}

#[test]
fn cannot_set_blocks_per_round_below_module_min() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 2u32),
			Error::<Test>::CannotSetBelowMin
		);
	});
}

#[test]
fn cannot_set_blocks_per_round_to_current_blocks_per_round() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 5u32),
			Error::<Test>::NoWritingSameValue
		);
	});
}

#[test]
fn round_immediately_jumps_if_current_duration_exceeds_new_blocks_per_round() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			// we can't lower the blocks per round because it must be above the number of
			// sequencers, and we can't lower the number of sequencers because it must be above
			// MinSelectedCandidates. so we first raise blocks per round, then lower it.
			assert_ok!(SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 10u32));

			roll_to(17);
			assert_ok!(SequencerStaking::set_blocks_per_round(RuntimeOrigin::root(), 6u32));
		});
}

// ~~ PUBLIC ~~

// JOIN CANDIDATES

#[test]
fn join_candidates_lock_balance() {
	ExtBuilder::default().with_balances(vec![(1, 10)]).build().execute_with(|| {
		assert_eq!(<Test as crate::Config>::Currency::free_balance(&1), 10);
		assert_ok!(SequencerStaking::join_candidates(RuntimeOrigin::signed(1), 0u32));
		assert_eq!(
			<Test as crate::Config>::Currency::ensure_can_withdraw(
				&1,
				10,
				WithdrawReasons::all(),
				0
			),
			Err(Module(ModuleError {
				index: 1,
				error: [1, 0, 0, 0],
				message: Some("LiquidityRestrictions")
			}))
		);
	});
}

#[test]
fn join_candidates_not_increases_total_staked() {
	ExtBuilder::default().with_balances(vec![(1, 10)]).build().execute_with(|| {
		assert_eq!(Total::<Test>::get(), 0);
		assert_ok!(SequencerStaking::join_candidates(RuntimeOrigin::signed(1), 0u32));
		assert_eq!(Total::<Test>::get(), 0);
	});
}

#[test]
fn join_candidates_creates_candidate_state() {
	ExtBuilder::default().with_balances(vec![(1, 10)]).build().execute_with(|| {
		assert!(CandidateInfo::<Test>::get(1).is_none());
		assert_ok!(SequencerStaking::join_candidates(RuntimeOrigin::signed(1), 0u32));
		let candidate_state = CandidateInfo::<Test>::get(1).expect("just joined => exists");
		assert_eq!(candidate_state.bond, 0u128);
	});
}

#[test]
fn join_candidates_adds_to_candidate_pool() {
	ExtBuilder::default().with_balances(vec![(1, 10)]).build().execute_with(|| {
		assert!(CandidatePool::<Test>::get().0.is_empty());
		assert_ok!(SequencerStaking::join_candidates(RuntimeOrigin::signed(1), 0u32));
		let candidate_pool = CandidatePool::<Test>::get();
		assert_eq!(candidate_pool.0[0].owner, 1);
		assert_eq!(candidate_pool.0[0].amount, 0);
	});
}

#[test]
fn cannot_join_candidates_if_candidate() {
	ExtBuilder::default()
		.with_balances(vec![(1, 1000)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::join_candidates(RuntimeOrigin::signed(1), 100u32),
				Error::<Test>::CandidateExists
			);
		});
}

#[test]
fn cannot_join_candidates_if_delegator() {
	ExtBuilder::default()
		.with_balances(vec![(1, 50)])
		.with_assets(
			vec![(0, 2, 20)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::join_candidates(RuntimeOrigin::signed(2), 1u32),
				Error::<Test>::DelegatorExists
			);
		});
}

#[test]
fn can_force_join_candidates_without_min_bond() {
	ExtBuilder::default().with_balances(vec![(1, 10)]).build().execute_with(|| {
		assert_ok!(SequencerStaking::force_join_candidates(RuntimeOrigin::root(), 1, 9, 100u32));
	});
}

#[test]
fn insufficient_join_candidates_weight_hint_fails() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20), (3, 20), (4, 20), (5, 20), (6, 20)])
		.with_candidates(vec![1, 2, 3, 4, 5])
		.build()
		.execute_with(|| {
			for i in 0..5 {
				assert_noop!(
					SequencerStaking::join_candidates(RuntimeOrigin::signed(6), i),
					Error::<Test>::TooLowCandidateCountWeightHintJoinCandidates
				);
			}
		});
}

#[test]
fn sufficient_join_candidates_weight_hint_succeeds() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 20),
			(2, 20),
			(3, 20),
			(4, 20),
			(5, 20),
			(6, 20),
			(7, 20),
			(8, 20),
			(9, 20),
		])
		.with_candidates(vec![1, 2, 3, 4, 5])
		.build()
		.execute_with(|| {
			let mut count = 5u32;
			for i in 6..10 {
				assert_ok!(SequencerStaking::join_candidates(RuntimeOrigin::signed(i), count));
				count += 1u32;
			}
		});
}

#[test]
fn join_candidates_fails_if_above_max_candidate_count() {
	let mut candidates = vec![];
	let mut balances = vec![];
	for i in 1..=crate::mock::MaxSequencerCandidates::get() {
		candidates.push(i as u64);
		balances.push((i as u64, 80));
	}

	let new_candidate = crate::mock::MaxSequencerCandidates::get() as u64 + 1;
	balances.push((new_candidate, 100));

	ExtBuilder::default()
		.with_balances(balances)
		.with_candidates(candidates)
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::join_candidates(
					RuntimeOrigin::signed(new_candidate),
					crate::mock::MaxSequencerCandidates::get(),
				),
				Error::<Test>::CandidateLimitReached,
			);
		});
}

// SCHEDULE LEAVE CANDIDATES

#[test]
fn leave_candidates_removes_candidate_from_candidate_pool() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_eq!(CandidatePool::<Test>::get().0.len(), 1);
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			assert!(CandidatePool::<Test>::get().0.is_empty());
		});
}

#[test]
fn cannot_leave_candidates_if_not_candidate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32),
			Error::<Test>::CandidateDNE
		);
	});
}

#[test]
fn cannot_leave_candidates_if_already_leaving_candidates() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			assert_noop!(
				SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32),
				Error::<Test>::CandidateAlreadyLeaving
			);
		});
}

#[test]
fn insufficient_leave_candidates_weight_hint_fails() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20), (3, 20), (4, 20), (5, 20)])
		.with_candidates(vec![1, 2, 3, 4, 5])
		.build()
		.execute_with(|| {
			for i in 1..6 {
				assert_noop!(
					SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(i), 4u32),
					Error::<Test>::TooLowCandidateCountToLeaveCandidates
				);
			}
		});
}

#[test]
fn enable_marking_offline_works() {
	ExtBuilder::default().with_balances(vec![(1, 20)]).build().execute_with(|| {
		assert_ok!(SequencerStaking::enable_marking_offline(RuntimeOrigin::root(), true));
		assert!(EnableMarkingOffline::<Test>::get());

		// Set to false now
		assert_ok!(SequencerStaking::enable_marking_offline(RuntimeOrigin::root(), false));
		assert!(!EnableMarkingOffline::<Test>::get());
	});
}

#[test]
fn enable_marking_offline_fails_bad_origin() {
	ExtBuilder::default().with_balances(vec![(1, 20)]).build().execute_with(|| {
		assert_noop!(
			SequencerStaking::enable_marking_offline(RuntimeOrigin::signed(1), true),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn notify_inactive_sequencer_works() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20), (3, 20), (4, 20), (5, 20)])
		.with_candidates(vec![1, 2, 3, 4, 5])
		.build()
		.execute_with(|| {
			// Enable killswitch
			<EnableMarkingOffline<Test>>::set(true);

			// Round 2
			roll_to_round_begin(2);
			// Finalize the first block of round 2
			SequencerStaking::on_finalize(5);

			// We don't produce blocks on round 3
			roll_to_round_begin(3);
			roll_blocks(1);

			// We don't produce blocks on round 4
			roll_to_round_begin(4);
			roll_blocks(1);

			// Round 6 - notify the sequencer as inactive
			roll_to_round_begin(6);
			roll_blocks(1);

			assert_eq!(<Test as crate::Config>::MaxOfflineRounds::get(), 1);
			assert_eq!(<Test as crate::Config>::RewardPaymentDelay::get(), 2);

			// Call 'notify_inactive_sequencer' extrinsic
			assert_ok!(SequencerStaking::notify_inactive_sequencer(RuntimeOrigin::signed(1), 1));
		});
}

#[test]
fn notify_inactive_sequencer_fails_too_low_sequencer_count() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20), (3, 20)])
		.with_candidates(vec![1, 2, 3])
		.build()
		.execute_with(|| {
			// Enable killswitch
			<EnableMarkingOffline<Test>>::set(true);

			// Round 4
			roll_to_round_begin(4);
			roll_blocks(1);

			// Call 'notify_inactive_sequencer' extrinsic
			assert_noop!(
				SequencerStaking::notify_inactive_sequencer(RuntimeOrigin::signed(1), 1),
				Error::<Test>::TooLowSequencerCountToNotifyAsInactive
			);
		});
}

#[test]
fn notify_inactive_sequencer_fails_round_too_low() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20), (3, 20), (4, 20), (5, 20)])
		.with_candidates(vec![1, 2, 3, 4, 5])
		.build()
		.execute_with(|| {
			// Enable killswitch
			<EnableMarkingOffline<Test>>::set(true);

			// Round 1
			roll_to_round_begin(1);
			roll_blocks(1);

			// Call 'notify_inactive_sequencer' extrinsic
			assert_noop!(
				SequencerStaking::notify_inactive_sequencer(RuntimeOrigin::signed(1), 1),
				Error::<Test>::CurrentRoundTooLow
			);
		});
}

#[test]
fn sufficient_leave_candidates_weight_hint_succeeds() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20), (3, 20), (4, 20), (5, 20)])
		.with_candidates(vec![1, 2, 3, 4, 5])
		.build()
		.execute_with(|| {
			let mut count = 5u32;
			for i in 1..6 {
				assert_ok!(SequencerStaking::schedule_leave_candidates(
					RuntimeOrigin::signed(i),
					count
				));
				count -= 1u32;
			}
		});
}

// EXECUTE LEAVE CANDIDATES

#[test]
fn execute_leave_candidates_callable_by_any_signed() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(2), 1, 0));
		});
}

#[test]
fn execute_leave_candidates_requires_correct_weight_hint() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10), (2, 10), (3, 10), (4, 10)])
		.with_assets(
			vec![(0, 1, 10), (0, 2, 10), (0, 3, 10), (0, 4, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10), (3, 1, 10), (4, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			roll_to(10);
			for i in 0..3 {
				assert_noop!(
					SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(1), 1, i),
					Error::<Test>::TooLowCandidateDelegationCountToLeaveCandidates
				);
			}
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(2), 1, 3));
		});
}

#[test]
fn execute_leave_candidates_unreserves_balance() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_eq!(<Test as crate::Config>::Currency::free_balance(&1), 10);
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(1), 1, 0));
			assert_eq!(<Test as crate::Config>::Currency::free_balance(&1), 10);
		});
}

#[test]
fn execute_leave_candidates_not_decreases_total_staked() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_eq!(Total::<Test>::get(), 0);
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(1), 1, 0));
			assert_eq!(Total::<Test>::get(), 0);
		});
}

#[test]
fn execute_leave_candidates_removes_candidate_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			// candidate state is not immediately removed
			let candidate_state = CandidateInfo::<Test>::get(1).expect("just left => still exists");
			assert_eq!(candidate_state.bond, 0);
			roll_to(10);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(1), 1, 0));
			assert!(CandidateInfo::<Test>::get(1).is_none());
		});
}

#[test]
fn execute_leave_candidates_removes_pending_delegation_requests() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10), (2, 15)])
		.with_assets(
			vec![(0, 2, 20)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 15)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5
			));
			let state = DelegationScheduledRequests::<Test>::get(&1);
			assert_eq!(
				state,
				vec![ScheduledRequest {
					delegator: 2,
					when_executable: 3,
					action: DelegationAction::Decrease(5),
				}],
			);
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			// candidate state is not immediately removed
			let candidate_state = CandidateInfo::<Test>::get(1).expect("just left => still exists");
			assert_eq!(candidate_state.bond, 0);
			roll_to(10);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(1), 1, 1));
			assert!(CandidateInfo::<Test>::get(1).is_none());
			assert!(
				!DelegationScheduledRequests::<Test>::get(&1).iter().any(|x| x.delegator == 2),
				"delegation request not removed"
			);
			assert!(
				!<DelegationScheduledRequests<Test>>::contains_key(&1),
				"the key was not removed from storage"
			);
		});
}

#[test]
fn cannot_execute_leave_candidates_before_delay() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			assert_noop!(
				SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(3), 1, 0)
					.map_err(|err| err.error),
				Error::<Test>::CandidateCannotLeaveYet
			);
			roll_to(9);
			assert_noop!(
				SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(3), 1, 0)
					.map_err(|err| err.error),
				Error::<Test>::CandidateCannotLeaveYet
			);
			roll_to(10);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(3), 1, 0));
		});
}

// CANCEL LEAVE CANDIDATES

#[test]
fn cancel_leave_candidates_updates_candidate_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			assert_ok!(SequencerStaking::cancel_leave_candidates(RuntimeOrigin::signed(1), 1));
			let candidate = CandidateInfo::<Test>::get(&1).expect("just cancelled leave so exists");
			assert!(candidate.is_active());
		});
}

#[test]
fn cancel_leave_candidates_adds_to_candidate_pool() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1u32));
			assert_ok!(SequencerStaking::cancel_leave_candidates(RuntimeOrigin::signed(1), 1));
			assert_eq!(CandidatePool::<Test>::get().0[0].owner, 1);
		});
}

// GO OFFLINE

#[test]
fn go_offline_removes_candidate_from_candidate_pool() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_eq!(CandidatePool::<Test>::get().0.len(), 1);
			assert_ok!(SequencerStaking::go_offline(RuntimeOrigin::signed(1)));
			assert!(CandidatePool::<Test>::get().0.is_empty());
		});
}

#[test]
fn go_offline_updates_candidate_state_to_idle() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			let candidate_state = CandidateInfo::<Test>::get(1).expect("is active candidate");
			assert_eq!(candidate_state.status, SequencerStatus::Active);
			assert_ok!(SequencerStaking::go_offline(RuntimeOrigin::signed(1)));
			let candidate_state =
				CandidateInfo::<Test>::get(1).expect("is candidate, just offline");
			assert_eq!(candidate_state.status, SequencerStatus::Idle);
		});
}

#[test]
fn cannot_go_offline_if_not_candidate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			SequencerStaking::go_offline(RuntimeOrigin::signed(3)).map_err(|err| err.error),
			Error::<Test>::CandidateDNE
		);
	});
}

#[test]
fn cannot_go_offline_if_already_offline() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::go_offline(RuntimeOrigin::signed(1)));
			assert_noop!(
				SequencerStaking::go_offline(RuntimeOrigin::signed(1)).map_err(|err| err.error),
				Error::<Test>::AlreadyOffline
			);
		});
}

// GO ONLINE

#[test]
fn go_online_adds_to_candidate_pool() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::go_offline(RuntimeOrigin::signed(1)));
			assert!(CandidatePool::<Test>::get().0.is_empty());
			assert_ok!(SequencerStaking::go_online(RuntimeOrigin::signed(1)));
			assert_eq!(CandidatePool::<Test>::get().0[0].owner, 1);
			assert_eq!(CandidatePool::<Test>::get().0[0].amount, 0);
		});
}

#[test]
fn go_online_storage_updates_candidate_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::go_offline(RuntimeOrigin::signed(1)));
			let candidate_state = CandidateInfo::<Test>::get(1).expect("offline still exists");
			assert_eq!(candidate_state.status, SequencerStatus::Idle);
			assert_ok!(SequencerStaking::go_online(RuntimeOrigin::signed(1)));
			let candidate_state = CandidateInfo::<Test>::get(1).expect("online so exists");
			assert_eq!(candidate_state.status, SequencerStatus::Active);
		});
}

#[test]
fn cannot_go_online_if_not_candidate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			SequencerStaking::go_online(RuntimeOrigin::signed(3)),
			Error::<Test>::CandidateDNE
		);
	});
}

#[test]
fn cannot_go_online_if_already_online() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::go_online(RuntimeOrigin::signed(1)).map_err(|err| err.error),
				Error::<Test>::AlreadyActive
			);
		});
}

#[test]
fn cannot_go_online_if_leaving() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1));
			assert_noop!(
				SequencerStaking::go_online(RuntimeOrigin::signed(1)).map_err(|err| err.error),
				Error::<Test>::CannotGoOnlineIfLeaving
			);
		});
}

// CANDIDATE BOND MORE

#[test]
fn candidate_bond_more_reserves_balance() {
	ExtBuilder::default()
		.with_balances(vec![(1, 50)])
		.with_assets(
			vec![(0, 1, 50)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&1,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				50
			);
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&1,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				20
			);
		});
}

#[test]
fn candidate_bond_more_increases_total() {
	ExtBuilder::default()
		.with_balances(vec![(1, 50)])
		.with_assets(
			vec![(0, 1, 50)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			let mut total = Total::<Test>::get();
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			total += 30;
			assert_eq!(Total::<Test>::get(), total);
		});
}

#[test]
fn candidate_bond_more_updates_candidate_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 50)])
		.with_assets(
			vec![(0, 1, 50)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			let candidate_state = CandidateInfo::<Test>::get(1).expect("updated => exists");
			assert_eq!(candidate_state.bond, 0);
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			let candidate_state = CandidateInfo::<Test>::get(1).expect("updated => exists");
			assert_eq!(candidate_state.bond, 30);
		});
}

#[test]
fn candidate_bond_more_updates_candidate_pool() {
	ExtBuilder::default()
		.with_balances(vec![(1, 50)])
		.with_assets(
			vec![(0, 1, 50)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_eq!(CandidatePool::<Test>::get().0[0].owner, 1);
			assert_eq!(CandidatePool::<Test>::get().0[0].amount, 0);
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_eq!(CandidatePool::<Test>::get().0[0].owner, 1);
			assert_eq!(CandidatePool::<Test>::get().0[0].amount, 30);
		});
}

// SCHEDULE CANDIDATE BOND LESS

#[test]
fn cannot_schedule_candidate_bond_less_if_request_exists() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_assets(
			vec![(0, 1, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			assert_ok!(SequencerStaking::schedule_candidate_bond_less(RuntimeOrigin::signed(1), 5));
			assert_noop!(
				SequencerStaking::schedule_candidate_bond_less(RuntimeOrigin::signed(1), 5),
				Error::<Test>::PendingCandidateRequestAlreadyExists
			);
		});
}

#[test]
fn cannot_schedule_candidate_bond_less_if_not_candidate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			SequencerStaking::schedule_candidate_bond_less(RuntimeOrigin::signed(6), 50),
			Error::<Test>::CandidateDNE
		);
	});
}

#[test]
fn cannot_schedule_candidate_bond_less_if_new_total_below_min_candidate_stk() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::schedule_candidate_bond_less(RuntimeOrigin::signed(1), 21),
				Error::<Test>::CandidateBondBelowMin
			);
		});
}

#[test]
fn can_schedule_candidate_bond_less_if_leaving_candidates() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_assets(
			vec![(0, 1, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1));
			assert_ok!(SequencerStaking::schedule_candidate_bond_less(
				RuntimeOrigin::signed(1),
				10
			));
		});
}

#[test]
fn cannot_schedule_candidate_bond_less_if_exited_candidates() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(1), 1, 0));
			assert_noop!(
				SequencerStaking::schedule_candidate_bond_less(RuntimeOrigin::signed(1), 10),
				Error::<Test>::CandidateDNE
			);
		});
}

// 2. EXECUTE BOND LESS REQUEST

#[test]
fn execute_candidate_bond_less_unreserves_balance() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_assets(
			vec![(0, 1, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_eq!(<Test as crate::Config>::Currency::free_balance(&1), 30);
			assert_ok!(SequencerStaking::schedule_candidate_bond_less(
				RuntimeOrigin::signed(1),
				10
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_candidate_bond_less(RuntimeOrigin::signed(1), 1));
			assert_eq!(<Test as crate::Config>::Currency::free_balance(&1), 30);
		});
}

#[test]
fn execute_candidate_bond_less_decreases_total() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_assets(
			vec![(0, 1, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			let mut total = Total::<Test>::get();
			assert_ok!(SequencerStaking::schedule_candidate_bond_less(
				RuntimeOrigin::signed(1),
				10
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_candidate_bond_less(RuntimeOrigin::signed(1), 1));
			total -= 10;
			assert_eq!(Total::<Test>::get(), total);
		});
}

#[test]
fn execute_candidate_bond_less_updates_candidate_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_assets(
			vec![(0, 1, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			let candidate_state = CandidateInfo::<Test>::get(1).expect("updated => exists");
			assert_eq!(candidate_state.bond, 30);
			assert_ok!(SequencerStaking::schedule_candidate_bond_less(
				RuntimeOrigin::signed(1),
				10
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_candidate_bond_less(RuntimeOrigin::signed(1), 1));
			let candidate_state = CandidateInfo::<Test>::get(1).expect("updated => exists");
			assert_eq!(candidate_state.bond, 20);
		});
}

#[test]
fn execute_candidate_bond_less_updates_candidate_pool() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_assets(
			vec![(0, 1, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_eq!(CandidatePool::<Test>::get().0[0].owner, 1);
			assert_eq!(CandidatePool::<Test>::get().0[0].amount, 30);
			assert_ok!(SequencerStaking::schedule_candidate_bond_less(
				RuntimeOrigin::signed(1),
				10
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_candidate_bond_less(RuntimeOrigin::signed(1), 1));
			assert_eq!(CandidatePool::<Test>::get().0[0].owner, 1);
			assert_eq!(CandidatePool::<Test>::get().0[0].amount, 20);
		});
}

// CANCEL CANDIDATE BOND LESS REQUEST

#[test]
fn cancel_candidate_bond_less_updates_candidate_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_assets(
			vec![(0, 1, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_ok!(SequencerStaking::schedule_candidate_bond_less(
				RuntimeOrigin::signed(1),
				10
			));
			assert_ok!(SequencerStaking::cancel_candidate_bond_less(RuntimeOrigin::signed(1)));
			assert!(CandidateInfo::<Test>::get(&1).unwrap().request.is_none());
		});
}

#[test]
fn only_candidate_can_cancel_candidate_bond_less_request() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30)])
		.with_assets(
			vec![(0, 1, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_ok!(SequencerStaking::schedule_candidate_bond_less(
				RuntimeOrigin::signed(1),
				10
			));
			assert_noop!(
				SequencerStaking::cancel_candidate_bond_less(RuntimeOrigin::signed(2)),
				Error::<Test>::CandidateDNE
			);
		});
}

// DELEGATE

#[test]
fn delegate_reserves_balance() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				10
			);
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(2), 1, 10, 0, 0));
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				0
			);
		});
}

#[test]
fn delegate_updates_delegator_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 10));
			assert!(DelegatorState::<Test>::get(2).is_none());
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(2), 1, 10, 0, 0));
			let delegator_state = DelegatorState::<Test>::get(2).expect("just delegated => exists");
			assert_eq!(delegator_state.total(), 10);
			assert_eq!(delegator_state.delegations.0[0].owner, 1);
			assert_eq!(delegator_state.delegations.0[0].amount, 10);
		});
}

#[test]
fn delegate_updates_sequencer_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			let candidate_state = CandidateInfo::<Test>::get(1).expect("registered in genesis");
			assert_eq!(candidate_state.total_counted, 30);
			let top_delegations = TopDelegations::<Test>::get(1).expect("registered in genesis");
			assert!(top_delegations.delegations.is_empty());
			assert!(top_delegations.total.is_zero());
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(2), 1, 10, 0, 0));
			let candidate_state = CandidateInfo::<Test>::get(1).expect("just delegated => exists");
			assert_eq!(candidate_state.total_counted, 40);
			let top_delegations = TopDelegations::<Test>::get(1).expect("just delegated => exists");
			assert_eq!(top_delegations.delegations[0].owner, 2);
			assert_eq!(top_delegations.delegations[0].amount, 10);
			assert_eq!(top_delegations.total, 10);
		});
}

#[test]
fn can_delegate_immediately_after_other_join_candidates() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20)])
		.with_assets(
			vec![(0, 2, 20)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::join_candidates(RuntimeOrigin::signed(1), 0));
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(2), 1, 20, 0, 0));
		});
}

#[test]
fn can_delegate_if_revoking() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 30), (3, 20), (4, 20)])
		.with_assets(
			vec![(0, 2, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3, 4])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(2), 4, 10, 0, 2));
		});
}

#[test]
fn cannot_delegate_if_full_and_new_delegation_less_than_or_equal_lowest_bottom() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 20),
			(2, 10),
			(3, 10),
			(4, 10),
			(5, 10),
			(6, 10),
			(7, 10),
			(8, 10),
			(9, 10),
			(10, 10),
			(11, 10),
		])
		.with_assets(
			vec![
				(0, 2, 10),
				(0, 3, 10),
				(0, 4, 10),
				(0, 5, 10),
				(0, 6, 10),
				(0, 7, 10),
				(0, 8, 10),
				(0, 9, 10),
				(0, 10, 10),
				(0, 11, 10),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![
			(2, 1, 10),
			(3, 1, 10),
			(4, 1, 10),
			(5, 1, 10),
			(6, 1, 10),
			(8, 1, 10),
			(9, 1, 10),
			(10, 1, 10),
		])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::delegate(RuntimeOrigin::signed(11), 1, 10, 8, 0),
				Error::<Test>::CannotDelegateLessThanOrEqualToLowestBottomWhenFull
			);
		});
}

#[test]
fn can_delegate_if_full_and_new_delegation_greater_than_lowest_bottom() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_assets(
			vec![
				(0, 2, 10),
				(0, 3, 10),
				(0, 4, 10),
				(0, 5, 10),
				(0, 6, 10),
				(0, 7, 10),
				(0, 8, 10),
				(0, 9, 10),
				(0, 10, 10),
				(0, 11, 11),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![
			(2, 1, 10),
			(3, 1, 10),
			(4, 1, 10),
			(5, 1, 10),
			(6, 1, 10),
			(8, 1, 10),
			(9, 1, 10),
			(10, 1, 10),
		])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(11), 1, 11, 8, 0));
		});
}

#[test]
fn can_still_delegate_if_leaving() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20), (3, 20)])
		.with_assets(
			vec![(0, 2, 20)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1,));
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(2), 3, 10, 0, 1),);
		});
}

#[test]
fn cannot_delegate_if_candidate() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 30)])
		.with_assets(
			vec![(0, 2, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 2])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::delegate(RuntimeOrigin::signed(2), 1, 10, 0, 0),
				Error::<Test>::CandidateExists
			);
		});
}

#[test]
fn cannot_delegate_if_already_delegated() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 30)])
		.with_assets(
			vec![(0, 2, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 20)])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::delegate(RuntimeOrigin::signed(2), 1, 10, 1, 1),
				Error::<Test>::AlreadyDelegatedCandidate
			);
		});
}

#[test]
fn cannot_delegate_more_than_max_delegations() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 50), (3, 20), (4, 20), (5, 20), (6, 20)])
		.with_assets(
			vec![(0, 2, 50)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3, 4, 5, 6])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10), (2, 4, 10), (2, 5, 10)])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::delegate(RuntimeOrigin::signed(2), 6, 10, 0, 4),
				Error::<Test>::ExceedMaxDelegationsPerDelegator,
			);
		});
}

#[test]
fn sufficient_delegate_weight_hint_succeeds() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 20),
			(2, 20),
			(3, 20),
			(4, 20),
			(5, 20),
			(6, 20),
			(7, 20),
			(8, 20),
			(9, 20),
			(10, 20),
		])
		.with_assets(
			vec![
				(0, 1, 20),
				(0, 2, 20),
				(0, 3, 20),
				(0, 4, 20),
				(0, 5, 20),
				(0, 6, 20),
				(0, 7, 20),
				(0, 8, 20),
				(0, 9, 20),
				(0, 10, 20),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 2])
		.with_delegations(vec![(3, 1, 10), (4, 1, 10), (5, 1, 10), (6, 1, 10)])
		.build()
		.execute_with(|| {
			let mut count = 4u32;
			for i in 7..11 {
				assert_ok!(SequencerStaking::delegate(
					RuntimeOrigin::signed(i),
					1,
					10,
					count,
					0u32
				));
				count += 1u32;
			}
			let mut count = 0u32;
			for i in 3..11 {
				assert_ok!(SequencerStaking::delegate(
					RuntimeOrigin::signed(i),
					2,
					10,
					count,
					1u32
				));
				count += 1u32;
			}
		});
}

#[test]
fn insufficient_delegate_weight_hint_fails() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 20),
			(2, 20),
			(3, 20),
			(4, 20),
			(5, 20),
			(6, 20),
			(7, 20),
			(8, 20),
			(9, 20),
			(10, 20),
		])
		.with_assets(
			vec![
				(0, 1, 20),
				(0, 2, 20),
				(0, 3, 20),
				(0, 4, 20),
				(0, 5, 20),
				(0, 6, 20),
				(0, 7, 20),
				(0, 8, 20),
				(0, 9, 20),
				(0, 10, 20),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 2])
		.with_delegations(vec![(3, 1, 10), (4, 1, 10), (5, 1, 10), (6, 1, 10)])
		.build()
		.execute_with(|| {
			let mut count = 3u32;
			for i in 7..11 {
				assert_noop!(
					SequencerStaking::delegate(RuntimeOrigin::signed(i), 1, 10, count, 0u32),
					Error::<Test>::TooLowCandidateDelegationCountToDelegate
				);
			}
			// to set up for next error test
			count = 4u32;
			for i in 7..11 {
				assert_ok!(SequencerStaking::delegate(
					RuntimeOrigin::signed(i),
					1,
					10,
					count,
					0u32
				));
				count += 1u32;
			}
			count = 0u32;
			for i in 3..11 {
				assert_noop!(
					SequencerStaking::delegate(RuntimeOrigin::signed(i), 2, 10, count, 0u32),
					Error::<Test>::TooLowDelegationCountToDelegate
				);
				count += 1u32;
			}
		});
}

// SCHEDULE REVOKE DELEGATION

#[test]
fn cannot_revoke_delegation_if_not_delegator() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1),
			Error::<Test>::DelegatorDNE
		);
	});
}

#[test]
fn cannot_revoke_delegation_that_dne() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 3),
				Error::<Test>::DelegationDNE
			);
		});
}

#[test]
fn can_schedule_revoke_delegation_below_min_delegator_stake() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 8), (3, 20)])
		.with_assets(
			vec![(0, 2, 8)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 5), (2, 3, 3)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
		});
}

// DELEGATOR BOND MORE

#[test]
fn delegator_bond_more_reserves_balance() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				5
			);
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 1, 5));
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				0
			);
		});
}

#[test]
fn delegator_bond_more_increases_total_staked() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_eq!(Total::<Test>::get(), 10);
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 1, 5));
			assert_eq!(Total::<Test>::get(), 15);
		});
}

#[test]
fn delegator_bond_more_updates_delegator_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 10));
			assert_eq!(DelegatorState::<Test>::get(2).expect("exists").total(), 10);
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 1, 5));
			assert_eq!(DelegatorState::<Test>::get(2).expect("exists").total(), 15);
		});
}

#[test]
fn delegator_bond_more_updates_candidate_state_top_delegations() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 10));
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().delegations[0].owner, 2);
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().delegations[0].amount, 10);
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().total, 10);
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 1, 5));
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().delegations[0].owner, 2);
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().delegations[0].amount, 15);
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().total, 15);
		});
}

#[test]
fn delegator_bond_more_updates_candidate_state_bottom_delegations() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 20), (3, 20), (4, 20), (5, 20), (6, 20)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 20), (0, 3, 20), (0, 4, 20), (0, 5, 20), (0, 6, 20)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10), (3, 1, 20), (4, 1, 20), (5, 1, 20), (6, 1, 20)])
		.build()
		.execute_with(|| {
			assert_eq!(BottomDelegations::<Test>::get(1).expect("exists").delegations[0].owner, 2);
			assert_eq!(
				BottomDelegations::<Test>::get(1).expect("exists").delegations[0].amount,
				10
			);
			assert_eq!(BottomDelegations::<Test>::get(1).unwrap().total, 10);
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 1, 5));
			assert_eq!(BottomDelegations::<Test>::get(1).expect("exists").delegations[0].owner, 2);
			assert_eq!(
				BottomDelegations::<Test>::get(1).expect("exists").delegations[0].amount,
				15
			);
			assert_eq!(BottomDelegations::<Test>::get(1).unwrap().total, 15);
		});
}

#[test]
fn delegator_bond_more_increases_total() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_eq!(Total::<Test>::get(), 10);
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 1, 5));
			assert_eq!(Total::<Test>::get(), 15);
		});
}

#[test]
fn can_delegator_bond_more_for_leaving_candidate() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1));
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 1, 5));
		});
}

#[test]
fn delegator_bond_more_disallowed_when_revoke_scheduled() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 25)])
		.with_assets(
			vec![(0, 2, 25)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			assert_noop!(
				SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 1, 5),
				<Error<Test>>::PendingDelegationRevoke
			);
		});
}

#[test]
fn delegator_bond_more_allowed_when_bond_decrease_scheduled() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 25)])
		.with_assets(
			vec![(0, 2, 25)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 15)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5,
			));
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 1, 5));
		});
}

// DELEGATOR BOND LESS

#[test]
fn delegator_bond_less_updates_delegator_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5
			));
			let state = DelegationScheduledRequests::<Test>::get(&1);
			assert_eq!(
				state,
				vec![ScheduledRequest {
					delegator: 2,
					when_executable: 3,
					action: DelegationAction::Decrease(5),
				}],
			);
		});
}

#[test]
fn cannot_delegator_bond_less_if_revoking() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 25), (3, 20)])
		.with_assets(
			vec![(0, 2, 25)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			assert_noop!(
				SequencerStaking::schedule_delegator_bond_less(RuntimeOrigin::signed(2), 1, 1)
					.map_err(|err| err.error),
				Error::<Test>::PendingDelegationRequestAlreadyExists
			);
		});
}

#[test]
fn cannot_delegator_bond_less_if_not_delegator() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			SequencerStaking::schedule_delegator_bond_less(RuntimeOrigin::signed(2), 1, 5)
				.map_err(|err| err.error),
			Error::<Test>::DelegatorDNE
		);
	});
}

#[test]
fn cannot_delegator_bond_less_if_candidate_dne() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::schedule_delegator_bond_less(RuntimeOrigin::signed(2), 3, 5)
					.map_err(|err| err.error),
				Error::<Test>::DelegationDNE
			);
		});
}

#[test]
fn cannot_delegator_bond_less_if_delegation_dne() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10), (3, 30)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::schedule_delegator_bond_less(RuntimeOrigin::signed(2), 3, 5)
					.map_err(|err| err.error),
				Error::<Test>::DelegationDNE
			);
		});
}

#[test]
fn cannot_delegator_bond_less_more_than_total_delegation() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::schedule_delegator_bond_less(RuntimeOrigin::signed(2), 1, 11)
					.map_err(|err| err.error),
				Error::<Test>::DelegatorBondBelowMin
			);
		});
}

#[test]
fn cannot_delegator_bond_less_below_min_delegation() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 20), (3, 30)])
		.with_assets(
			vec![(0, 2, 20)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_noop!(
				SequencerStaking::schedule_delegator_bond_less(RuntimeOrigin::signed(2), 1, 8)
					.map_err(|err| err.error),
				Error::<Test>::DelegationBelowMin
			);
		});
}

// EXECUTE PENDING DELEGATION REQUEST

// 1. REVOKE DELEGATION

#[test]
fn execute_revoke_delegation_unreserves_balance() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				0
			);
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				10
			);
		});
}

#[test]
fn execute_revoke_delegation_adds_revocation_to_delegator_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 20), (3, 20)])
		.with_assets(
			vec![(0, 2, 20)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert!(!DelegationScheduledRequests::<Test>::get(&1).iter().any(|x| x.delegator == 2));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			assert!(DelegationScheduledRequests::<Test>::get(&1).iter().any(|x| x.delegator == 2));
		});
}

#[test]
fn execute_revoke_delegation_removes_revocation_from_delegator_state_upon_execution() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 20), (3, 20)])
		.with_assets(
			vec![(0, 2, 20)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert!(!DelegationScheduledRequests::<Test>::get(&1).iter().any(|x| x.delegator == 2));
		});
}

#[test]
fn execute_revoke_delegation_removes_revocation_from_state_for_single_delegation_leave() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 20), (3, 20)])
		.with_assets(
			vec![(0, 2, 20)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert!(
				!DelegationScheduledRequests::<Test>::get(&1).iter().any(|x| x.delegator == 2),
				"delegation was not removed"
			);
		});
}

#[test]
fn execute_revoke_delegation_decreases_total_staked() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_eq!(Total::<Test>::get(), 40);
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert_eq!(Total::<Test>::get(), 30);
		});
}

#[test]
fn execute_revoke_delegation_for_last_delegation_removes_delegator_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert!(DelegatorState::<Test>::get(2).is_some());
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			roll_to(10);
			// this will be confusing for people
			// if status is leaving, then execute_delegation_request works if last delegation
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert!(DelegatorState::<Test>::get(2).is_none());
		});
}

#[test]
fn execute_revoke_delegation_removes_delegation_from_candidate_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_eq!(CandidateInfo::<Test>::get(1).expect("exists").delegation_count, 1u32);
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert!(CandidateInfo::<Test>::get(1).expect("exists").delegation_count.is_zero());
		});
}

#[test]
fn can_execute_revoke_delegation_for_leaving_candidate() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			roll_to(10);
			// can execute delegation request for leaving candidate
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
		});
}

#[test]
fn can_execute_leave_candidates_if_revoking_candidate() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			roll_to(10);
			// revocation executes during execute leave candidates (callable by anyone)
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(1), 1, 1));
			assert!(!SequencerStaking::is_delegator(&2));
			assert_eq!(Balances::reserved_balance(&2), 0);
			assert_eq!(Balances::free_balance(&2), 10);
		});
}

#[test]
fn delegator_bond_more_after_revoke_delegation_does_not_effect_exit() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 30), (3, 30)])
		.with_assets(
			vec![(0, 2, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(2), 3, 10));
			roll_to(100);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert!(SequencerStaking::is_delegator(&2));
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				10
			);
		});
}

#[test]
fn delegator_bond_less_after_revoke_delegation_does_not_effect_exit() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 30), (3, 30)])
		.with_assets(
			vec![(0, 2, 30)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			assert_noop!(
				SequencerStaking::schedule_delegator_bond_less(RuntimeOrigin::signed(2), 1, 2)
					.map_err(|err| err.error),
				Error::<Test>::PendingDelegationRequestAlreadyExists
			);
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				3,
				2
			));
			roll_to(10);
			roll_blocks(1);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				3
			));
			assert!(SequencerStaking::is_delegator(&2));
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				22
			);
		});
}

// 2. EXECUTE BOND LESS

#[test]
fn execute_delegator_bond_less_unreserves_balance() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				0
			);
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&2,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				5
			);
		});
}

#[test]
fn execute_delegator_bond_less_decreases_total_staked() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_eq!(Total::<Test>::get(), 10);
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert_eq!(Total::<Test>::get(), 5);
		});
}

#[test]
fn execute_delegator_bond_less_updates_delegator_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 10));
			assert_eq!(DelegatorState::<Test>::get(2).expect("exists").total(), 10);
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert_eq!(DelegatorState::<Test>::get(2).expect("exists").total(), 5);
		});
}

#[test]
fn execute_delegator_bond_less_updates_candidate_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 10));
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().delegations[0].owner, 2);
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().delegations[0].amount, 10);
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().delegations[0].owner, 2);
			assert_eq!(TopDelegations::<Test>::get(1).unwrap().delegations[0].amount, 5);
		});
}

#[test]
fn execute_delegator_bond_less_decreases_total() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_eq!(Total::<Test>::get(), 10);
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert_eq!(Total::<Test>::get(), 5);
		});
}

#[test]
fn execute_delegator_bond_less_updates_just_bottom_delegations() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 10), (3, 11), (4, 12), (5, 14), (6, 15)])
		.with_assets(
			vec![(0, 1, 20), (0, 2, 10), (0, 3, 11), (0, 4, 12), (0, 5, 14), (0, 6, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10), (3, 1, 11), (4, 1, 12), (5, 1, 14), (6, 1, 15)])
		.build()
		.execute_with(|| {
			let pre_call_candidate_info =
				CandidateInfo::<Test>::get(&1).expect("delegated by all so exists");
			let pre_call_top_delegations =
				TopDelegations::<Test>::get(&1).expect("delegated by all so exists");
			let pre_call_bottom_delegations =
				BottomDelegations::<Test>::get(&1).expect("delegated by all so exists");
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				2
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			let post_call_candidate_info =
				CandidateInfo::<Test>::get(&1).expect("delegated by all so exists");
			let post_call_top_delegations =
				TopDelegations::<Test>::get(&1).expect("delegated by all so exists");
			let post_call_bottom_delegations =
				BottomDelegations::<Test>::get(&1).expect("delegated by all so exists");
			let mut not_equal = false;
			for Bond { owner, amount } in pre_call_bottom_delegations.delegations {
				for Bond { owner: post_owner, amount: post_amount } in
					&post_call_bottom_delegations.delegations
				{
					if &owner == post_owner {
						if &amount != post_amount {
							not_equal = true;
							break;
						}
					}
				}
			}
			assert!(not_equal);
			let mut equal = true;
			for Bond { owner, amount } in pre_call_top_delegations.delegations {
				for Bond { owner: post_owner, amount: post_amount } in
					&post_call_top_delegations.delegations
				{
					if &owner == post_owner {
						if &amount != post_amount {
							equal = false;
							break;
						}
					}
				}
			}
			assert!(equal);
			assert_eq!(
				pre_call_candidate_info.total_counted,
				post_call_candidate_info.total_counted
			);
		});
}

#[test]
fn execute_delegator_bond_less_does_not_delete_bottom_delegations() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 10), (3, 11), (4, 12), (5, 14), (6, 15)])
		.with_assets(
			vec![(0, 1, 20), (0, 2, 10), (0, 3, 11), (0, 4, 12), (0, 5, 14), (0, 6, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10), (3, 1, 11), (4, 1, 12), (5, 1, 14), (6, 1, 15)])
		.build()
		.execute_with(|| {
			let pre_call_candidate_info =
				CandidateInfo::<Test>::get(&1).expect("delegated by all so exists");
			let pre_call_top_delegations =
				TopDelegations::<Test>::get(&1).expect("delegated by all so exists");
			let pre_call_bottom_delegations =
				BottomDelegations::<Test>::get(&1).expect("delegated by all so exists");
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(6),
				1,
				4
			));
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(6),
				6,
				1
			));
			let post_call_candidate_info =
				CandidateInfo::<Test>::get(&1).expect("delegated by all so exists");
			let post_call_top_delegations =
				TopDelegations::<Test>::get(&1).expect("delegated by all so exists");
			let post_call_bottom_delegations =
				BottomDelegations::<Test>::get(&1).expect("delegated by all so exists");
			let mut equal = true;
			for Bond { owner, amount } in pre_call_bottom_delegations.delegations {
				for Bond { owner: post_owner, amount: post_amount } in
					&post_call_bottom_delegations.delegations
				{
					if &owner == post_owner {
						if &amount != post_amount {
							equal = false;
							break;
						}
					}
				}
			}
			assert!(equal);
			let mut not_equal = false;
			for Bond { owner, amount } in pre_call_top_delegations.delegations {
				for Bond { owner: post_owner, amount: post_amount } in
					&post_call_top_delegations.delegations
				{
					if &owner == post_owner {
						if &amount != post_amount {
							not_equal = true;
							break;
						}
					}
				}
			}
			assert!(not_equal);
			assert_eq!(
				pre_call_candidate_info.total_counted - 4,
				post_call_candidate_info.total_counted
			);
		});
}

#[test]
fn can_execute_delegator_bond_less_for_leaving_candidate() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 1, 30), (0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 15)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 30));
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(1), 1));
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5
			));
			roll_to(10);
			// can execute bond more delegation request for leaving candidate
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
		});
}

// CANCEL PENDING DELEGATION REQUEST
// 1. CANCEL REVOKE DELEGATION

#[test]
fn cancel_revoke_delegation_updates_delegator_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 10)])
		.with_assets(
			vec![(0, 2, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			let state = DelegationScheduledRequests::<Test>::get(&1);
			assert_eq!(
				state,
				vec![ScheduledRequest {
					delegator: 2,
					when_executable: 3,
					action: DelegationAction::Revoke(10),
				}],
			);
			assert_eq!(
				DelegatorState::<Test>::get(&2)
					.map(|x| x.less_total)
					.expect("delegator state must exist"),
				10
			);
			assert_ok!(SequencerStaking::cancel_delegation_request(RuntimeOrigin::signed(2), 1));
			assert!(!DelegationScheduledRequests::<Test>::get(&1).iter().any(|x| x.delegator == 2));
			assert_eq!(
				DelegatorState::<Test>::get(&2)
					.map(|x| x.less_total)
					.expect("delegator state must exist"),
				0
			);
		});
}

// 2. CANCEL DELEGATOR BOND LESS

#[test]
fn cancel_delegator_bond_less_updates_delegator_state() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 15)])
		.with_assets(
			vec![(0, 2, 15)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 15)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				5
			));
			let state = DelegationScheduledRequests::<Test>::get(&1);
			assert_eq!(
				state,
				vec![ScheduledRequest {
					delegator: 2,
					when_executable: 3,
					action: DelegationAction::Decrease(5),
				}],
			);
			assert_eq!(
				DelegatorState::<Test>::get(&2)
					.map(|x| x.less_total)
					.expect("delegator state must exist"),
				5
			);
			assert_ok!(SequencerStaking::cancel_delegation_request(RuntimeOrigin::signed(2), 1));
			assert!(!DelegationScheduledRequests::<Test>::get(&1).iter().any(|x| x.delegator == 2));
			assert_eq!(
				DelegatorState::<Test>::get(&2)
					.map(|x| x.less_total)
					.expect("delegator state must exist"),
				0
			);
		});
}

// ~~ PROPERTY-BASED TESTS ~~

#[test]
fn delegator_schedule_revocation_total() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 40), (3, 20), (4, 20), (5, 20)])
		.with_assets(
			vec![(0, 2, 40)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3, 4, 5])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10), (2, 4, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			assert_eq!(
				DelegatorState::<Test>::get(&2)
					.map(|x| x.less_total)
					.expect("delegator state must exist"),
				10
			);
			roll_to(10);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				1
			));
			assert_eq!(
				DelegatorState::<Test>::get(&2)
					.map(|x| x.less_total)
					.expect("delegator state must exist"),
				0
			);
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(2), 5, 10, 0, 2));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 3));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 4));
			assert_eq!(
				DelegatorState::<Test>::get(&2)
					.map(|x| x.less_total)
					.expect("delegator state must exist"),
				20,
			);
			roll_to(20);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				3
			));
			assert_eq!(
				DelegatorState::<Test>::get(&2)
					.map(|x| x.less_total)
					.expect("delegator state must exist"),
				10,
			);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(2),
				2,
				4
			));
			assert_eq!(
				DelegatorState::<Test>::get(&2)
					.map(|x| x.less_total)
					.expect("delegator state must exist"),
				0
			);
		});
}

#[ignore]
#[test]
fn parachain_bond_inflation_reserve_matches_config() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 100),
			(2, 100),
			(3, 100),
			(4, 100),
			(5, 100),
			(6, 100),
			(7, 100),
			(8, 100),
			(9, 100),
			(10, 100),
			(11, 1),
		])
		.with_candidates(vec![1, 2, 3, 4, 5])
		.with_delegations(vec![(6, 1, 10), (7, 1, 10), (8, 2, 10), (9, 2, 10), (10, 1, 10)])
		.build()
		.execute_with(|| {
			assert_eq!(Balances::free_balance(&11), 1);
			roll_to_round_begin(2);

			assert_eq!(Balances::free_balance(&11), 1);
			roll_to_round_begin(4);
			// distribute total issuance to sequencer 1 and its delegators 6, 7, 19
			assert_eq!(Balances::free_balance(&11), 16);
			// 1. ensure delegators are paid for 2 rounds after they leave
			assert_noop!(
				SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(66), 1),
				Error::<Test>::DelegatorDNE
			);
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(6), 1,));

			roll_blocks(3);

			// fast forward to block in which delegator 6 exit executes
			roll_to_round_begin(5);

			roll_blocks(3);

			roll_to_round_begin(6);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(6),
				6,
				10
			));

			roll_blocks(3);

			roll_to_round_begin(7);

			roll_blocks(3);

			assert_eq!(Balances::free_balance(&11), 65);

			// 6 won't be paid for this round because they left already
			roll_to_round_begin(8);
			roll_blocks(3);
			assert_eq!(Balances::free_balance(&11), 95);

			roll_to_round_begin(9);
			roll_blocks(3);
			assert_eq!(Balances::free_balance(&11), 127);

			roll_blocks(1);
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(8), 1, 10, 10, 10));

			roll_to_round_begin(10);
			roll_blocks(3);
			assert_eq!(Balances::free_balance(&11), 160);

			roll_to_round_begin(11);
			roll_blocks(3);
			assert_eq!(Balances::free_balance(&11), 195);

			roll_to_round_begin(12);
			roll_blocks(3);
			assert_eq!(Balances::free_balance(&11), 232);
		});
}

#[test]
fn sequencer_exit_executes_after_delay() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 1000),
			(2, 300),
			(3, 100),
			(4, 100),
			(5, 100),
			(6, 100),
			(7, 100),
			(8, 9),
			(9, 4),
		])
		.with_assets(
			vec![(0, 2, 300), (0, 3, 100), (0, 4, 100), (0, 5, 100), (0, 6, 100)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 2])
		.with_delegations(vec![(3, 1, 100), (4, 1, 100), (5, 2, 100), (6, 2, 100)])
		.build()
		.execute_with(|| {
			roll_to(11);
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(2), 2));

			let info = CandidateInfo::<Test>::get(&2).unwrap();
			assert_eq!(info.status, SequencerStatus::Leaving(5));
			roll_to(21);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(2), 2, 2));
		});
}

#[test]
fn sequencer_selection_chooses_top_candidates() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 1000),
			(2, 1000),
			(3, 1000),
			(4, 1000),
			(5, 1000),
			(6, 1000),
			(7, 33),
			(8, 33),
			(9, 33),
		])
		.with_candidates(vec![1, 2, 3, 4, 5, 6])
		.build()
		.execute_with(|| {
			roll_to_round_begin(2);
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(6), 6));

			roll_to_round_begin(4);
			roll_blocks(1);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(6), 6, 0));
			assert_ok!(SequencerStaking::join_candidates(RuntimeOrigin::signed(6), 100u32));
		});
}

#[test]
fn multiple_delegations() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 100),
			(2, 100),
			(3, 100),
			(4, 100),
			(5, 100),
			(6, 100),
			(7, 100),
			(8, 100),
			(9, 100),
			(10, 100),
		])
		.with_assets(
			vec![
				(0, 1, 100),
				(0, 2, 100),
				(0, 3, 100),
				(0, 4, 100),
				(0, 5, 100),
				(0, 6, 100),
				(0, 7, 100),
				(0, 8, 100),
				(0, 9, 100),
				(0, 10, 100),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 2, 3, 4, 5])
		.with_delegations(vec![(6, 1, 10), (7, 1, 10), (8, 2, 10), (9, 2, 10), (10, 1, 10)])
		.build()
		.execute_with(|| {
			roll_to_round_begin(2);
			// chooses top TotalSelectedCandidates (5), in order
			roll_blocks(1);
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(6), 2, 10, 10, 10));
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(6), 3, 10, 10, 10));
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(6), 4, 10, 10, 10));

			roll_to_round_begin(6);
			roll_blocks(1);
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(7), 2, 80, 10, 10));
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(10), 2, 10, 10, 10));
			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(2), 5));

			roll_to_round_begin(7);

			// verify that delegations are removed after sequencer leaves, not before
			assert_eq!(DelegatorState::<Test>::get(7).unwrap().total(), 90);
			assert_eq!(DelegatorState::<Test>::get(7).unwrap().delegations.0.len(), 2usize);
			assert_eq!(DelegatorState::<Test>::get(6).unwrap().total(), 40);
			assert_eq!(DelegatorState::<Test>::get(6).unwrap().delegations.0.len(), 4usize);
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&6,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				60
			);
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&7,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				10
			);
			roll_to_round_begin(8);
			roll_blocks(1);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(2), 2, 5));

			assert_eq!(DelegatorState::<Test>::get(7).unwrap().total(), 10);
			assert_eq!(DelegatorState::<Test>::get(6).unwrap().total(), 30);
			assert_eq!(DelegatorState::<Test>::get(7).unwrap().delegations.0.len(), 1usize);
			assert_eq!(DelegatorState::<Test>::get(6).unwrap().delegations.0.len(), 3usize);
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&6,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				70
			);
			assert_eq!(
				<Test as crate::Config>::Assets::reducible_balance(
					<Test as crate::Config>::BTC::get(),
					&7,
					Preservation::Expendable,
					Fortitude::Polite,
				),
				90
			);
		});
}

#[test]
// The test verifies that the pending revoke request is removed by 2's exit so there is no
// dangling revoke request after 2 exits
fn execute_leave_candidate_removes_delegations() {
	ExtBuilder::default()
		.with_balances(vec![(1, 100), (2, 100), (3, 100), (4, 100)])
		.with_assets(
			vec![(0, 2, 100), (0, 3, 100), (0, 4, 100)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 2])
		.with_delegations(vec![(3, 1, 10), (3, 2, 10), (4, 1, 10), (4, 2, 10)])
		.build()
		.execute_with(|| {
			// Verifies the revocation request is initially empty
			assert!(!DelegationScheduledRequests::<Test>::get(&2).iter().any(|x| x.delegator == 3));

			assert_ok!(SequencerStaking::schedule_leave_candidates(RuntimeOrigin::signed(2), 2));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(3), 2));
			// Verifies the revocation request is present
			assert!(DelegationScheduledRequests::<Test>::get(&2).iter().any(|x| x.delegator == 3));

			roll_to(16);
			assert_ok!(SequencerStaking::execute_leave_candidates(RuntimeOrigin::signed(2), 2, 2));
			// Verifies the revocation request is again empty
			assert!(!DelegationScheduledRequests::<Test>::get(&2).iter().any(|x| x.delegator == 3));
		});
}

#[test]
fn bottom_delegations_are_empty_when_top_delegations_not_full() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 10), (3, 10), (4, 10), (5, 10)])
		.with_assets(
			vec![(0, 1, 20), (0, 2, 10), (0, 3, 10), (0, 4, 10), (0, 5, 10)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			// no top delegators => no bottom delegators
			let top_delegations = TopDelegations::<Test>::get(1).unwrap();
			let bottom_delegations = BottomDelegations::<Test>::get(1).unwrap();
			assert!(top_delegations.delegations.is_empty());
			assert!(bottom_delegations.delegations.is_empty());
			// 1 delegator => 1 top delegator, 0 bottom delegators
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(2), 1, 10, 10, 10));
			let top_delegations = TopDelegations::<Test>::get(1).unwrap();
			let bottom_delegations = BottomDelegations::<Test>::get(1).unwrap();
			assert_eq!(top_delegations.delegations.len(), 1usize);
			assert!(bottom_delegations.delegations.is_empty());
			// 2 delegators => 2 top delegators, 0 bottom delegators
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(3), 1, 10, 10, 10));
			let top_delegations = TopDelegations::<Test>::get(1).unwrap();
			let bottom_delegations = BottomDelegations::<Test>::get(1).unwrap();
			assert_eq!(top_delegations.delegations.len(), 2usize);
			assert!(bottom_delegations.delegations.is_empty());
			// 3 delegators => 3 top delegators, 0 bottom delegators
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(4), 1, 10, 10, 10));
			let top_delegations = TopDelegations::<Test>::get(1).unwrap();
			let bottom_delegations = BottomDelegations::<Test>::get(1).unwrap();
			assert_eq!(top_delegations.delegations.len(), 3usize);
			assert!(bottom_delegations.delegations.is_empty());
			// 4 delegators => 4 top delegators, 0 bottom delegators
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(5), 1, 10, 10, 10));
			let top_delegations = TopDelegations::<Test>::get(1).unwrap();
			let bottom_delegations = BottomDelegations::<Test>::get(1).unwrap();
			assert_eq!(top_delegations.delegations.len(), 4usize);
			assert!(bottom_delegations.delegations.is_empty());
		});
}

#[test]
fn candidate_pool_updates_when_total_counted_changes() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 20),
			(3, 19),
			(4, 20),
			(5, 21),
			(6, 22),
			(7, 15),
			(8, 16),
			(9, 17),
			(10, 18),
		])
		.with_assets(
			vec![
				(0, 1, 20),
				(0, 3, 19),
				(0, 4, 20),
				(0, 5, 21),
				(0, 6, 22),
				(0, 7, 15),
				(0, 8, 16),
				(0, 9, 17),
				(0, 10, 18),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![
			(3, 1, 11),
			(4, 1, 12),
			(5, 1, 13),
			(6, 1, 14),
			(7, 1, 15),
			(8, 1, 16),
			(9, 1, 17),
			(10, 1, 18),
		])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			fn is_candidate_pool_bond(account: u64, bond: u128) {
				let pool = CandidatePool::<Test>::get();
				for candidate in pool.0 {
					if candidate.owner == account {
						assert_eq!(
							candidate.amount, bond,
							"Candidate Bond {:?} is Not Equal to Expected: {:?}",
							candidate.amount, bond
						);
					}
				}
			}
			// 15 + 16 + 17 + 18 + 20 = 86 (top 4 + self bond)
			is_candidate_pool_bond(1, 86);
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(3), 1, 8));
			// 3: 11 -> 19 => 3 is in top, bumps out 7
			// 16 + 17 + 18 + 19 + 20 = 90 (top 4 + self bond)
			is_candidate_pool_bond(1, 90);
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(4), 1, 8));
			// 4: 12 -> 20 => 4 is in top, bumps out 8
			// 17 + 18 + 19 + 20 + 20 = 94 (top 4 + self bond)
			is_candidate_pool_bond(1, 94);
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(10),
				1,
				3
			));
			roll_to(30);
			// 10: 18 -> 15 => 10 bumped to bottom, 8 bumped to top (- 18 + 16 = -2 for count)
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(10),
				10,
				1
			));
			// 16 + 17 + 19 + 20 + 20 = 92 (top 4 + self bond)
			is_candidate_pool_bond(1, 92);
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(9),
				1,
				4
			));
			roll_to(40);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(9),
				9,
				1
			));
			// 15 + 16 + 19 + 20 + 20 = 90 (top 4 + self bond)
			is_candidate_pool_bond(1, 90);
		});
}

#[test]
fn only_top_sequencers_are_counted() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 20),
			(3, 19),
			(4, 20),
			(5, 21),
			(6, 22),
			(7, 15),
			(8, 16),
			(9, 17),
			(10, 18),
		])
		.with_assets(
			vec![
				(0, 1, 20),
				(0, 3, 19),
				(0, 4, 20),
				(0, 5, 21),
				(0, 6, 22),
				(0, 7, 15),
				(0, 8, 16),
				(0, 9, 17),
				(0, 10, 18),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![
			(3, 1, 11),
			(4, 1, 12),
			(5, 1, 13),
			(6, 1, 14),
			(7, 1, 15),
			(8, 1, 16),
			(9, 1, 17),
			(10, 1, 18),
		])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			// sanity check that 3-10 are delegators immediately
			for i in 3..11 {
				assert!(SequencerStaking::is_delegator(&i));
			}
			let sequencer_state = CandidateInfo::<Test>::get(1).unwrap();
			// 15 + 16 + 17 + 18 + 20 = 86 (top 4 + self bond)
			assert_eq!(sequencer_state.total_counted, 86);
			// bump bottom to the top
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(3), 1, 8));

			let sequencer_state = CandidateInfo::<Test>::get(1).unwrap();
			// 16 + 17 + 18 + 19 + 20 = 90 (top 4 + self bond)
			assert_eq!(sequencer_state.total_counted, 90);
			// bump bottom to the top
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(4), 1, 8));

			let sequencer_state = CandidateInfo::<Test>::get(1).unwrap();
			// 17 + 18 + 19 + 20 + 20 = 94 (top 4 + self bond)
			assert_eq!(sequencer_state.total_counted, 94);
			// bump bottom to the top
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(5), 1, 8));

			let sequencer_state = CandidateInfo::<Test>::get(1).unwrap();
			// 18 + 19 + 20 + 21 + 20 = 98 (top 4 + self bond)
			assert_eq!(sequencer_state.total_counted, 98);
			// bump bottom to the top
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(6), 1, 8));

			let sequencer_state = CandidateInfo::<Test>::get(1).unwrap();
			// 19 + 20 + 21 + 22 + 20 = 102 (top 4 + self bond)
			assert_eq!(sequencer_state.total_counted, 102);
		});
}

#[test]
fn delegation_events_convey_correct_position() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 100),
			(2, 100),
			(3, 100),
			(4, 100),
			(5, 100),
			(6, 100),
			(7, 100),
			(8, 100),
			(9, 100),
			(10, 100),
		])
		.with_assets(
			vec![
				(0, 1, 100),
				(0, 2, 100),
				(0, 3, 100),
				(0, 4, 100),
				(0, 5, 100),
				(0, 6, 100),
				(0, 7, 100),
				(0, 8, 100),
				(0, 9, 100),
				(0, 10, 100),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 2])
		.with_delegations(vec![(3, 1, 11), (4, 1, 12), (5, 1, 13), (6, 1, 14)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			let sequencer1_state = CandidateInfo::<Test>::get(1).unwrap();
			// 11 + 12 + 13 + 14 + 20 = 70 (top 4 + self bond)
			assert_eq!(sequencer1_state.total_counted, 70);
			// Top delegations are full, new highest delegation is made
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(7), 1, 15, 10, 10));

			let sequencer1_state = CandidateInfo::<Test>::get(1).unwrap();
			// 12 + 13 + 14 + 15 + 20 = 70 (top 4 + self bond)
			assert_eq!(sequencer1_state.total_counted, 74);
			// New delegation is added to the bottom
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(8), 1, 10, 10, 10));

			let sequencer1_state = CandidateInfo::<Test>::get(1).unwrap();
			// 12 + 13 + 14 + 15 + 20 = 70 (top 4 + self bond)
			assert_eq!(sequencer1_state.total_counted, 74);
			// 8 increases delegation to the top
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(8), 1, 3));

			let sequencer1_state = CandidateInfo::<Test>::get(1).unwrap();
			// 13 + 13 + 14 + 15 + 20 = 75 (top 4 + self bond)
			assert_eq!(sequencer1_state.total_counted, 75);
			// 3 increases delegation but stays in bottom
			assert_ok!(SequencerStaking::delegator_bond_more(RuntimeOrigin::signed(3), 1, 1));

			let sequencer1_state = CandidateInfo::<Test>::get(1).unwrap();
			// 13 + 13 + 14 + 15 + 20 = 75 (top 4 + self bond)
			assert_eq!(sequencer1_state.total_counted, 75);
			// 6 decreases delegation but stays in top
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(6),
				1,
				2
			));

			roll_to(30);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(6),
				6,
				1
			));

			let sequencer1_state = CandidateInfo::<Test>::get(1).unwrap();
			// 12 + 13 + 13 + 15 + 20 = 73 (top 4 + self bond)ƒ
			assert_eq!(sequencer1_state.total_counted, 73);
			// 6 decreases delegation and is bumped to bottom
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(6),
				1,
				1
			));

			roll_to(40);
			assert_ok!(SequencerStaking::execute_delegation_request(
				RuntimeOrigin::signed(6),
				6,
				1
			));

			let sequencer1_state = CandidateInfo::<Test>::get(1).unwrap();
			// 12 + 13 + 13 + 15 + 20 = 73 (top 4 + self bond)
			assert_eq!(sequencer1_state.total_counted, 73);
		});
}

#[test]
fn deferred_payment_storage_items_are_cleaned_up() {
	use crate::*;

	// this test sets up two sequencers, gives them points in round one, and focuses on the
	// storage over the next several blocks to show that it is properly cleaned up

	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20)])
		.with_candidates(vec![1, 2])
		.build()
		.execute_with(|| {
			AwardedPts::<Test>::set(1, 1, 1);
			AwardedPts::<Test>::set(1, 2, 1);
			// 1+1 = 2
			<Points<Test>>::insert(1, 2);

			// reflects genesis?
			assert!(<AtStake<Test>>::contains_key(1, 1));
			assert!(<AtStake<Test>>::contains_key(1, 2));

			roll_to_round_begin(2);

			// we should have AtStake snapshots as soon as we start a round...
			assert!(<AtStake<Test>>::contains_key(2, 1));
			assert!(<AtStake<Test>>::contains_key(2, 2));
			// ...and it should persist until the round is fully paid out
			assert!(<AtStake<Test>>::contains_key(1, 1));
			assert!(<AtStake<Test>>::contains_key(1, 2));

			assert!(
				<Points<Test>>::contains_key(1),
				"Points should be populated during current round"
			);

			assert!(
				!<Points<Test>>::contains_key(2),
				"Points should not be populated until author noted"
			);

			// first payout occurs in round 3
			roll_to_round_begin(3);
			roll_blocks(1);

			// payouts should exist for past rounds that haven't been paid out yet..
			assert!(<AtStake<Test>>::contains_key(3, 1));
			assert!(<AtStake<Test>>::contains_key(3, 2));
			assert!(<AtStake<Test>>::contains_key(2, 1));
			assert!(<AtStake<Test>>::contains_key(2, 2));

			assert!(
				<DelayedPayouts<Test>>::contains_key(1),
				"DelayedPayouts should be populated after RewardPaymentDelay"
			);
			assert!(<Points<Test>>::contains_key(1));
			assert!(!<DelayedPayouts<Test>>::contains_key(2));
			assert!(!<Points<Test>>::contains_key(2), "We never rewarded points for round 2");

			assert!(!<DelayedPayouts<Test>>::contains_key(3));
			assert!(!<Points<Test>>::contains_key(3), "We never awarded points for round 3");

			// sequencer 1 has been paid in this last block and associated storage cleaned up
			assert!(!<AtStake<Test>>::contains_key(1, 1));
			assert!(!<AwardedPts<Test>>::contains_key(1, 1));

			// but sequencer 2 hasn't been paid
			assert!(<AtStake<Test>>::contains_key(1, 2));
			assert!(<AwardedPts<Test>>::contains_key(1, 2));

			// second payout occurs in next block
			roll_blocks(1);
			roll_to_round_begin(4);

			// sequencers have both been paid and storage fully cleaned up for round 1
			assert!(!<AtStake<Test>>::contains_key(1, 2));
			assert!(!<AwardedPts<Test>>::contains_key(1, 2));
			assert!(!<Points<Test>>::contains_key(1)); // points should be cleaned up
			assert!(!<DelayedPayouts<Test>>::contains_key(1));
		});
}

#[test]
fn deferred_payment_and_at_stake_storage_items_cleaned_up_for_candidates_not_producing_blocks() {
	use crate::*;

	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 20), (3, 20)])
		.with_candidates(vec![1, 2, 3])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 2u32, 3u32));
			AwardedPts::<Test>::set(1, 1, 1);
			AwardedPts::<Test>::set(1, 2, 1);
			// 1+1 = 2
			<Points<Test>>::insert(1, 2);

			// reflects genesis?
			assert!(<AtStake<Test>>::contains_key(1, 1));
			assert!(<AtStake<Test>>::contains_key(1, 2));

			roll_to_round_begin(2);
			assert!(<AtStake<Test>>::contains_key(1, 1));
			assert!(<AtStake<Test>>::contains_key(1, 2));
			assert!(<AtStake<Test>>::contains_key(1, 3));
			assert!(<AwardedPts<Test>>::contains_key(1, 1));
			assert!(<AwardedPts<Test>>::contains_key(1, 2));
			assert!(!<AwardedPts<Test>>::contains_key(1, 3));
			assert!(<Points<Test>>::contains_key(1));
			roll_to_round_begin(3);
			assert!(<DelayedPayouts<Test>>::contains_key(1));

			// all storage items must be cleaned up
			roll_to_round_begin(4);
			assert!(!<AtStake<Test>>::contains_key(1, 1));
			assert!(!<AtStake<Test>>::contains_key(1, 2));
			assert!(!<AtStake<Test>>::contains_key(1, 3));
			assert!(!<AwardedPts<Test>>::contains_key(1, 1));
			assert!(!<AwardedPts<Test>>::contains_key(1, 2));
			assert!(!<AwardedPts<Test>>::contains_key(1, 3));
			assert!(!<Points<Test>>::contains_key(1));
			assert!(!<DelayedPayouts<Test>>::contains_key(1));
		});
}

#[test]
fn deferred_payment_steady_state_event_flow() {
	use frame_support::traits::{Currency, ExistenceRequirement, WithdrawReasons};

	// this test "flows" through a number of rounds, asserting that certain things do/don't happen
	// once the staking pallet is in a "steady state" (specifically, once we are past the first few
	// rounds to clear RewardPaymentDelay)

	ExtBuilder::default()
		.with_balances(vec![
			// sequencers
			(1, 200),
			(2, 200),
			(3, 200),
			(4, 200),
			// delegators
			(11, 200),
			(22, 200),
			(33, 200),
			(44, 200),
			// burn account, see `reset_issuance()`
			(111, 1000),
		])
		.with_assets(
			vec![
				(0, 1, 200),
				(0, 2, 200),
				(0, 3, 200),
				(0, 4, 200),
				(0, 11, 200),
				(0, 22, 200),
				(0, 33, 200),
				(0, 44, 200),
				(0, 111, 1000),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 2, 3, 4])
		.with_delegations(vec![
			// delegator 11 delegates 100 to 1 and 2
			(11, 1, 100),
			(11, 2, 100),
			// delegator 22 delegates 100 to 2 and 3
			(22, 2, 100),
			(22, 3, 100),
			// delegator 33 delegates 100 to 3 and 4
			(33, 3, 100),
			(33, 4, 100),
			// delegator 44 delegates 100 to 4 and 1
			(44, 4, 100),
			(44, 1, 100),
		])
		.build()
		.execute_with(|| {
			// grab initial issuance -- we will reset it before round issuance is calculated so that
			// it is consistent every round
			let initial_issuance = Balances::total_issuance();
			let reset_issuance = || {
				let new_issuance = Balances::total_issuance();
				let diff = new_issuance - initial_issuance;
				let burned = Balances::burn(diff);
				Balances::settle(
					&111,
					burned,
					WithdrawReasons::FEE,
					ExistenceRequirement::AllowDeath,
				)
				.expect("Account can absorb burn");
			};

			// fn to roll through the first RewardPaymentDelay rounds. returns new round index
			let roll_through_initial_rounds = |mut round: BlockNumber| -> BlockNumber {
				while round < (crate::mock::RewardPaymentDelay::get() + 1).into() {
					roll_to_round_end(round);
					round += 1;
				}

				reset_issuance();

				round
			};

			// roll through a "steady state" round and make all of our assertions
			// returns new round index
			let roll_through_steady_state_round = |round: BlockNumber| -> BlockNumber {
				let num_rounds_rolled = roll_to_round_begin(round);
				assert!(num_rounds_rolled <= 1, "expected to be at round begin already");

				roll_blocks(5);
				// Since we defer first deferred staking payout, this test have the maximum amout of
				// supported sequencers. This eman that the next round is trigerred one block after
				// the last reward.

				let num_rounds_rolled = roll_to_round_end(round);
				assert_eq!(num_rounds_rolled, 0, "expected to be at round end already");

				reset_issuance();

				round + 1
			};

			let mut round = 1;
			round = roll_through_initial_rounds(round); // we should be at RewardPaymentDelay
			for _ in 1..2 {
				round = roll_through_steady_state_round(round);
			}
		});
}

#[test]
fn delegation_kicked_from_bottom_removes_pending_request() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 30),
			(2, 29),
			(3, 20),
			(4, 20),
			(5, 20),
			(6, 20),
			(7, 20),
			(8, 20),
			(9, 20),
			(10, 20),
			(11, 30),
		])
		.with_assets(
			vec![
				(0, 1, 30),
				(0, 2, 29),
				(0, 3, 20),
				(0, 4, 20),
				(0, 5, 20),
				(0, 6, 20),
				(0, 7, 20),
				(0, 8, 20),
				(0, 9, 20),
				(0, 10, 20),
				(0, 11, 30),
			],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 11])
		.with_delegations(vec![
			(2, 1, 19),
			(2, 11, 10), // second delegation so not left after first is kicked
			(3, 1, 20),
			(4, 1, 20),
			(5, 1, 20),
			(6, 1, 20),
			(7, 1, 20),
			(8, 1, 20),
			(9, 1, 20),
		])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));
			// 10 delegates to full 1 => kicks lowest delegation (2, 19)
			assert_ok!(SequencerStaking::delegate(RuntimeOrigin::signed(10), 1, 20, 8, 0));

			// ensure request DNE
			assert!(!DelegationScheduledRequests::<Test>::get(&1).iter().any(|x| x.delegator == 2));
		});
}

#[test]
fn no_selected_candidates_defaults_to_last_round_sequencers() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 30), (3, 30), (4, 30), (5, 30)])
		.with_candidates(vec![1, 2, 3, 4, 5])
		.build()
		.execute_with(|| {
			roll_to_round_begin(1);
			// schedule to leave
			for i in 1..6 {
				assert_ok!(SequencerStaking::schedule_leave_candidates(
					RuntimeOrigin::signed(i),
					5
				));
			}
			let old_round = Round::<Test>::get().current;
			let old_selected_candidates = SelectedCandidates::<Test>::get();
			let mut old_at_stake_snapshots = Vec::new();
			for account in old_selected_candidates.clone() {
				old_at_stake_snapshots.push(<AtStake<Test>>::get(old_round, account));
			}
			roll_to_round_begin(3);
			// execute leave
			for i in 1..6 {
				assert_ok!(SequencerStaking::execute_leave_candidates(
					RuntimeOrigin::signed(i),
					i,
					0,
				));
			}
			// next round
			roll_to_round_begin(4);
			let new_round = Round::<Test>::get().current;
			// check AtStake matches previous
			let new_selected_candidates = SelectedCandidates::<Test>::get();
			assert_eq!(old_selected_candidates, new_selected_candidates);
			let mut index = 0usize;
			for account in new_selected_candidates {
				assert_eq!(old_at_stake_snapshots[index], <AtStake<Test>>::get(new_round, account));
				index += 1usize;
			}
		});
}

#[test]
fn test_delegator_scheduled_for_revoke_is_rewarded_for_previous_rounds_but_not_for_future() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 40), (3, 20), (4, 20)])
		.with_assets(
			vec![(0, 1, 20), (0, 2, 40)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3, 4])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));

			let sequencer = CandidateInfo::<Test>::get(1).expect("candidate must exist");
			assert_eq!(
				1, sequencer.delegation_count,
				"sequencer's delegator count was reduced unexpectedly"
			);
			assert_eq!(30, sequencer.total_counted, "sequencer's total was reduced unexpectedly");

			roll_to_round_begin(3);
			roll_blocks(3);
			roll_to_round_begin(4);
			roll_blocks(3);
			let sequencer_snapshot =
				AtStake::<Test>::get(Round::<Test>::get().current, 1).unwrap_or_default();
			assert_eq!(
				1,
				sequencer_snapshot.delegations.len(),
				"sequencer snapshot's delegator count was reduced unexpectedly"
			);
			assert_eq!(
				20, sequencer_snapshot.total,
				"sequencer snapshot's total was reduced unexpectedly",
			);
		});
}

#[test]
fn test_delegator_scheduled_for_revoke_is_rewarded_when_request_cancelled() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 40), (3, 20), (4, 20)])
		.with_assets(
			vec![(0, 1, 20), (0, 2, 40)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3, 4])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1));

			let sequencer = CandidateInfo::<Test>::get(1).expect("candidate must exist");
			assert_eq!(
				1, sequencer.delegation_count,
				"sequencer's delegator count was reduced unexpectedly"
			);
			assert_eq!(30, sequencer.total_counted, "sequencer's total was reduced unexpectedly");

			roll_to_round_begin(2);
			assert_ok!(SequencerStaking::cancel_delegation_request(RuntimeOrigin::signed(2), 1));

			roll_to_round_begin(4);
			roll_blocks(3);

			let sequencer_snapshot =
				AtStake::<Test>::get(Round::<Test>::get().current, 1).unwrap_or_default();
			assert_eq!(
				1,
				sequencer_snapshot.delegations.len(),
				"sequencer snapshot's delegator count was reduced unexpectedly"
			);
			assert_eq!(
				30, sequencer_snapshot.total,
				"sequencer snapshot's total was reduced unexpectedly",
			);
		});
}

#[test]
fn test_delegator_scheduled_for_bond_decrease_is_rewarded_for_previous_rounds_but_less_for_future()
{
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 40), (3, 20), (4, 20)])
		.with_assets(
			vec![(0, 1, 20), (0, 2, 40)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3, 4])
		.with_delegations(vec![(2, 1, 20), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				10,
			));

			let sequencer = CandidateInfo::<Test>::get(1).expect("candidate must exist");
			assert_eq!(
				1, sequencer.delegation_count,
				"sequencer's delegator count was reduced unexpectedly"
			);
			assert_eq!(40, sequencer.total_counted, "sequencer's total was reduced unexpectedly");

			roll_to_round_begin(3);
			roll_blocks(3);

			roll_to_round_begin(4);
			roll_blocks(3);

			let sequencer_snapshot =
				AtStake::<Test>::get(Round::<Test>::get().current, 1).unwrap_or_default();
			assert_eq!(
				1,
				sequencer_snapshot.delegations.len(),
				"sequencer snapshot's delegator count was reduced unexpectedly"
			);
			assert_eq!(
				30, sequencer_snapshot.total,
				"sequencer snapshot's total was reduced unexpectedly",
			);
		});
}

#[test]
fn test_delegator_scheduled_for_bond_decrease_is_rewarded_when_request_cancelled() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 40), (3, 20), (4, 20)])
		.with_assets(
			vec![(0, 1, 20), (0, 2, 40)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3, 4])
		.with_delegations(vec![(2, 1, 20), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			assert_ok!(SequencerStaking::schedule_delegator_bond_less(
				RuntimeOrigin::signed(2),
				1,
				10,
			));

			let sequencer = CandidateInfo::<Test>::get(1).expect("candidate must exist");
			assert_eq!(
				1, sequencer.delegation_count,
				"sequencer's delegator count was reduced unexpectedly"
			);
			assert_eq!(40, sequencer.total_counted, "sequencer's total was reduced unexpectedly");

			roll_to_round_begin(2);
			assert_ok!(SequencerStaking::cancel_delegation_request(RuntimeOrigin::signed(2), 1));

			roll_to_round_begin(4);
			roll_blocks(3);

			let sequencer_snapshot =
				AtStake::<Test>::get(Round::<Test>::get().current, 1).unwrap_or_default();
			assert_eq!(
				1,
				sequencer_snapshot.delegations.len(),
				"sequencer snapshot's delegator count was reduced unexpectedly"
			);
			assert_eq!(
				40, sequencer_snapshot.total,
				"sequencer snapshot's total was reduced unexpectedly",
			);
		});
}

#[test]
fn test_delegator_scheduled_for_leave_is_rewarded_for_previous_rounds_but_not_for_future() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 40), (3, 20), (4, 20)])
		.with_assets(
			vec![(0, 1, 50), (0, 2, 40)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3, 4])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1,));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 3,));

			let sequencer = CandidateInfo::<Test>::get(1).expect("candidate must exist");
			assert_eq!(
				1, sequencer.delegation_count,
				"sequencer's delegator count was reduced unexpectedly"
			);
			assert_eq!(30, sequencer.total_counted, "sequencer's total was reduced unexpectedly");

			roll_to_round_begin(3);
			roll_blocks(3);

			roll_to_round_begin(4);
			roll_blocks(3);

			let sequencer_snapshot =
				AtStake::<Test>::get(Round::<Test>::get().current, 1).unwrap_or_default();
			assert_eq!(
				1,
				sequencer_snapshot.delegations.len(),
				"sequencer snapshot's delegator count was reduced unexpectedly"
			);
			assert_eq!(
				20, sequencer_snapshot.total,
				"sequencer snapshot's total was reduced unexpectedly",
			);
		});
}

#[test]
fn test_delegator_scheduled_for_leave_is_rewarded_when_request_cancelled() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20), (2, 40), (3, 20), (4, 20)])
		.with_assets(
			vec![(0, 1, 20), (0, 2, 40)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1, 3, 4])
		.with_delegations(vec![(2, 1, 10), (2, 3, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(SequencerStaking::candidate_bond_more(RuntimeOrigin::signed(1), 20));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 1,));
			assert_ok!(SequencerStaking::schedule_revoke_delegation(RuntimeOrigin::signed(2), 3,));

			let sequencer = CandidateInfo::<Test>::get(1).expect("candidate must exist");
			assert_eq!(
				1, sequencer.delegation_count,
				"sequencer's delegator count was reduced unexpectedly"
			);
			assert_eq!(30, sequencer.total_counted, "sequencer's total was reduced unexpectedly");

			roll_to_round_begin(2);
			assert_ok!(SequencerStaking::cancel_delegation_request(RuntimeOrigin::signed(2), 1,));
			assert_ok!(SequencerStaking::cancel_delegation_request(RuntimeOrigin::signed(2), 3,));

			roll_to_round_begin(4);
			roll_blocks(3);

			let sequencer_snapshot =
				AtStake::<Test>::get(Round::<Test>::get().current, 1).unwrap_or_default();
			assert_eq!(
				1,
				sequencer_snapshot.delegations.len(),
				"sequencer snapshot's delegator count was reduced unexpectedly"
			);
			assert_eq!(
				30, sequencer_snapshot.total,
				"sequencer snapshot's total was reduced unexpectedly",
			);
		});
}

#[test]
fn test_delegation_request_exists_returns_false_when_nothing_exists() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 25)])
		.with_assets(
			vec![(0, 2, 25)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert!(!SequencerStaking::delegation_request_exists(&1, &2));
		});
}

#[test]
fn test_delegation_request_exists_returns_true_when_decrease_exists() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 25)])
		.with_assets(
			vec![(0, 2, 25)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			<DelegationScheduledRequests<Test>>::insert(
				1,
				BoundedVec::try_from(vec![ScheduledRequest {
					delegator: 2,
					when_executable: 3,
					action: DelegationAction::Decrease(5),
				}])
				.expect("must succeed"),
			);
			assert!(SequencerStaking::delegation_request_exists(&1, &2));
		});
}

#[test]
fn test_delegation_request_exists_returns_true_when_revoke_exists() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 25)])
		.with_assets(
			vec![(0, 2, 25)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			<DelegationScheduledRequests<Test>>::insert(
				1,
				BoundedVec::try_from(vec![ScheduledRequest {
					delegator: 2,
					when_executable: 3,
					action: DelegationAction::Revoke(5),
				}])
				.expect("must succeed"),
			);
			assert!(SequencerStaking::delegation_request_exists(&1, &2));
		});
}

#[test]
fn test_delegation_request_revoke_exists_returns_false_when_nothing_exists() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 25)])
		.with_assets(
			vec![(0, 2, 25)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			assert!(!SequencerStaking::delegation_request_revoke_exists(&1, &2));
		});
}

#[test]
fn test_delegation_request_revoke_exists_returns_false_when_decrease_exists() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 25)])
		.with_assets(
			vec![(0, 2, 25)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			<DelegationScheduledRequests<Test>>::insert(
				1,
				BoundedVec::try_from(vec![ScheduledRequest {
					delegator: 2,
					when_executable: 3,
					action: DelegationAction::Decrease(5),
				}])
				.expect("must succeed"),
			);
			assert!(!SequencerStaking::delegation_request_revoke_exists(&1, &2));
		});
}

#[test]
fn test_delegation_request_revoke_exists_returns_true_when_revoke_exists() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 25)])
		.with_assets(
			vec![(0, 2, 25)],
			// create asset BTC
			vec![(0, 1, true, 1)],
			// Metadata for BTC
			vec![(0, b"BTC".to_vec(), b"BTC".to_vec(), 8)],
		)
		.with_candidates(vec![1])
		.with_delegations(vec![(2, 1, 10)])
		.build()
		.execute_with(|| {
			<DelegationScheduledRequests<Test>>::insert(
				1,
				BoundedVec::try_from(vec![ScheduledRequest {
					delegator: 2,
					when_executable: 3,
					action: DelegationAction::Revoke(5),
				}])
				.expect("must succeed"),
			);
			assert!(SequencerStaking::delegation_request_revoke_exists(&1, &2));
		});
}

#[test]
fn test_hotfix_remove_delegation_requests_exited_candidates_cleans_up() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			// invalid state
			<DelegationScheduledRequests<Test>>::insert(2, BoundedVec::default());
			<DelegationScheduledRequests<Test>>::insert(3, BoundedVec::default());
			assert_ok!(SequencerStaking::hotfix_remove_delegation_requests_exited_candidates(
				RuntimeOrigin::signed(1),
				vec![2, 3, 4] // 4 does not exist, but is OK for idempotency
			));

			assert!(!<DelegationScheduledRequests<Test>>::contains_key(2));
			assert!(!<DelegationScheduledRequests<Test>>::contains_key(3));
		});
}

#[test]
fn test_hotfix_remove_delegation_requests_exited_candidates_cleans_up_only_specified_keys() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			// invalid state
			<DelegationScheduledRequests<Test>>::insert(2, BoundedVec::default());
			<DelegationScheduledRequests<Test>>::insert(3, BoundedVec::default());
			assert_ok!(SequencerStaking::hotfix_remove_delegation_requests_exited_candidates(
				RuntimeOrigin::signed(1),
				vec![2]
			));

			assert!(!<DelegationScheduledRequests<Test>>::contains_key(2));
			assert!(<DelegationScheduledRequests<Test>>::contains_key(3));
		});
}

#[test]
fn test_hotfix_remove_delegation_requests_exited_candidates_errors_when_requests_not_empty() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			// invalid state
			<DelegationScheduledRequests<Test>>::insert(2, BoundedVec::default());
			<DelegationScheduledRequests<Test>>::insert(
				3,
				BoundedVec::try_from(vec![ScheduledRequest {
					delegator: 10,
					when_executable: 1,
					action: DelegationAction::Revoke(10),
				}])
				.expect("must succeed"),
			);

			assert_noop!(
				SequencerStaking::hotfix_remove_delegation_requests_exited_candidates(
					RuntimeOrigin::signed(1),
					vec![2, 3]
				),
				<Error<Test>>::CandidateNotLeaving,
			);
		});
}

#[test]
fn test_hotfix_remove_delegation_requests_exited_candidates_errors_when_candidate_not_exited() {
	ExtBuilder::default()
		.with_balances(vec![(1, 20)])
		.with_candidates(vec![1])
		.build()
		.execute_with(|| {
			// invalid state
			<DelegationScheduledRequests<Test>>::insert(1, BoundedVec::default());
			assert_noop!(
				SequencerStaking::hotfix_remove_delegation_requests_exited_candidates(
					RuntimeOrigin::signed(1),
					vec![1]
				),
				<Error<Test>>::CandidateNotLeaving,
			);
		});
}

#[test]
fn test_compute_top_candidates_is_stable() {
	ExtBuilder::default()
		.with_balances(vec![(1, 30), (2, 30), (3, 30), (4, 30), (5, 30), (6, 30)])
		.with_candidates(vec![1, 2, 3, 4, 5, 6])
		.build()
		.execute_with(|| {
			// There are 6 candidates with equal amount, but only 5 can be selected
			assert_eq!(CandidatePool::<Test>::get().0.len(), 6);
			assert_eq!(<Test as crate::Config>::SequencerGroup::total_selected(), 5);
			// Returns the 5 candidates with greater AccountId, because they are iterated in reverse
			assert_eq!(SequencerStaking::compute_top_candidates(), vec![2, 3, 4, 5, 6]);
		});
}
