use crate::{mock::*, Event};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works_for_set_group_metric() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_ok!(SequencerGrouping::set_group_metric(RuntimeOrigin::root(), 3, 5));
	});
}
