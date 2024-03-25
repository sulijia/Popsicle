use crate::{mock::*, Event, SequencerGroup, Error, GroupMembers, NextRound};
use frame_support::{assert_noop, assert_ok};
use frame_support::pallet_prelude::Get;
use sp_runtime::testing::H256;
use sp_runtime::traits::BadOrigin;
use crate::pallet::{GroupNumber, GroupSize};

#[test]
fn it_works_for_set_group_metric() {
	new_test_ext().execute_with(|| {
		let group_size = 3;
		let group_number = 5;
		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), group_size, group_number));
		assert_eq!(GroupSize::<Test>::get(), 3);
		assert_eq!(GroupNumber::<Test>::get(), 5);
	});
}

#[test]
fn non_root_set_group_metric_fails() {
	new_test_ext().execute_with(|| {
		let group_size = 3;
		let group_number = 5;
		let non_root = 0;
		assert_noop!(
			SequencerGrouping::set_group_metric(RuntimeOrigin::signed(non_root), group_size, group_number),
			BadOrigin
		);
	});
}

#[test]
fn set_group_metric_fails_group_size_too_large() {
	new_test_ext().execute_with(|| {
		let group_size: u32 = <Test as crate::Config>::MaxGroupSize::get();
		let group_number: u32 = <Test as crate::Config>::MaxGroupNumber::get();
		assert_noop!(
			SequencerGrouping::set_group_metric(RuntimeOrigin::root(), group_size + 1, group_number + 1),
			Error::<Test>::GroupSizeTooLarge
		);
	});
}

#[test]
fn trigger_group_fails_candidates_not_enough() {
	new_test_ext().execute_with(|| {
		let starting_block = 20;
		let round_index = 2;
		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 3, 5));
		assert_noop!(
			SequencerGrouping::trigger_group(vec![1, 2], starting_block, round_index),
			Error::<Test>::CandidatesNotEnough
		);
	});
}

#[test]
fn trigger_group_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(10);
		let parent_hash = H256::from_low_u64_be(12345);
		frame_system::Pallet::<Test>::set_parent_hash(parent_hash);

		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 2, 3));
		assert_ok!(SequencerGrouping::trigger_group(vec![1, 2, 3, 4, 5, 6], 20, 3));
		System::assert_last_event(RuntimeEvent::SequencerGrouping(Event::SequencerGroupUpdated {
			starting_block: 20,
			round_index: 3,
		}));
		println!("Group Members: {:?}", GroupMembers::<Test>::get());

		System::set_block_number(11);
		let parent_hash = H256::from_low_u64_be(54321);
		frame_system::Pallet::<Test>::set_parent_hash(parent_hash);
		assert_ok!(SequencerGrouping::trigger_group(vec![1, 2, 3, 4, 5, 6], 30, 2));
		System::assert_last_event(RuntimeEvent::SequencerGrouping(Event::SequencerGroupUpdated {
			starting_block: 30,
			round_index: 2,
		}));
		println!("Group Members: {:?}", GroupMembers::<Test>::get());
	});
}

#[test]
fn account_in_group_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(10);
		let parent_hash = H256::from_low_u64_be(12345);
		frame_system::Pallet::<Test>::set_parent_hash(parent_hash);

		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 2, 3));
		assert_ok!(SequencerGrouping::trigger_group(vec![1, 2, 3, 4, 5, 6], 1, 1));
		println!("Group Members: {:?}", GroupMembers::<Test>::get());

		assert_eq!(SequencerGrouping::account_in_group(1), Ok(0));
		assert_eq!(SequencerGrouping::account_in_group(2), Ok(2));
		assert_eq!(SequencerGrouping::account_in_group(3), Ok(2));
		assert_eq!(SequencerGrouping::account_in_group(4), Ok(0));
		assert_eq!(SequencerGrouping::account_in_group(5), Ok(1));
		assert_eq!(SequencerGrouping::account_in_group(6), Ok(1));
	});
}

#[test]
fn account_in_group_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(10);
		let parent_hash = H256::from_low_u64_be(12345);
		frame_system::Pallet::<Test>::set_parent_hash(parent_hash);

		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 2, 3));
		assert_ok!(SequencerGrouping::trigger_group(vec![1, 2, 3, 4, 5, 6], 1, 1));
		println!("Group Members: {:?}", GroupMembers::<Test>::get());

		assert_noop!(SequencerGrouping::account_in_group(7), Error::<Test>::AccountNotInGroup);
	});
}

#[test]
fn all_group_ids_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(10);
		let parent_hash = H256::from_low_u64_be(12345);
		frame_system::Pallet::<Test>::set_parent_hash(parent_hash);

		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 2, 3));
		assert_ok!(SequencerGrouping::trigger_group(vec![1, 2, 3, 4, 5, 6], 15, 2));
		println!("Group Members: {:?}", GroupMembers::<Test>::get());

		assert_eq!(SequencerGrouping::all_group_ids(), vec![0, 1, 2]);
	});
}

#[test]
fn get_next_round_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 2, 3));
		assert_ok!(SequencerGrouping::trigger_group(vec![1, 2, 3, 4, 5, 6], 16, 3));
		println!("Group Members: {:?}", GroupMembers::<Test>::get());

		assert_eq!(SequencerGrouping::next_round(), NextRound {
			starting_block: 16,
			round_index: 3,
		});
	});
}

