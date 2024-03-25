use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use sp_core::H256;
use sp_runtime::BoundedVec;

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_ok!(ContainerModule::set_default_url(
			RuntimeOrigin::root(),
			BoundedVec::try_from("http://127.0.0.1:8000/static".as_bytes().to_vec()).unwrap()
		));
		// Read pallet storage and assert an expected result.
		assert_eq!(
			ContainerModule::default_url(),
			Some(BoundedVec::try_from("http://127.0.0.1:8000/static".as_bytes().to_vec()).unwrap())
		);
	});
}

#[test]
fn register_app() {
	new_test_ext().execute_with(|| {
		assert_ok!(ContainerModule::register_app(
			RuntimeOrigin::signed(1),
			H256::from([1; 32]),
			BoundedVec::try_from("test".as_bytes().to_vec()).unwrap(),
			123,
			BoundedVec::try_from("--chain dev".as_bytes().to_vec()).unwrap(),
			None
		));
		let app = ContainerModule::appinfo_map(1).unwrap();
		assert_eq!(app.app_hash, H256::from([1; 32]));
	});
}
