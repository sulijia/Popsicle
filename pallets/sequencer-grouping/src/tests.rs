use crate::{mock::*, Event, SequencerGroup, Error, GroupMembers};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::testing::H256;
use sp_runtime::traits::BadOrigin;
use crate::pallet::{GroupNumber, GroupSize};

#[test]
fn it_works_for_set_group_metric() {
	new_test_ext().execute_with(|| {
		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 3, 5));
		assert_eq!(GroupSize::<Test>::get(), 3);
		assert_eq!(GroupNumber::<Test>::get(), 5);
	});
}

#[test]
fn non_root_set_group_metric_fails() {
	new_test_ext().execute_with(|| {
		let non_root = 0;
		assert_noop!(
			SequencerGrouping::set_group_metric(RuntimeOrigin::signed(non_root), 3, 5),
			BadOrigin
		);
	});
}

#[test]
fn trigger_group_fails_candidates_not_enough() {
	new_test_ext().execute_with(|| {
		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 3, 5));
		assert_noop!(
			SequencerGrouping::trigger_group(vec![1, 2], 1, 1),
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
		assert_ok!(SequencerGrouping::trigger_group(vec![1, 2, 3, 4, 5, 6], 1, 1));
		System::assert_last_event(RuntimeEvent::SequencerGrouping(Event::SequencerGroupUpdated {
			starting_block: 1,
			round_index: 1,
		}));
		println!("Group Members: {:?}", GroupMembers::<Test>::get());

		System::set_block_number(11);
		let parent_hash = H256::from_low_u64_be(54321);
		frame_system::Pallet::<Test>::set_parent_hash(parent_hash);
		assert_ok!(SequencerGrouping::trigger_group(vec![1, 2, 3, 4, 5, 6], 2, 2));
		System::assert_last_event(RuntimeEvent::SequencerGrouping(Event::SequencerGroupUpdated {
			starting_block: 2,
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
		assert_ok!(SequencerGrouping::trigger_group(vec![1, 2, 3, 4, 5, 6], 1, 1));
		println!("Group Members: {:?}", GroupMembers::<Test>::get());

		assert_eq!(SequencerGrouping::all_group_ids(), vec![0, 1, 2]);
	});
}

