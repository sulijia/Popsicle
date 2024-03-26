//! Benchmarking setup for pallet-container

use super::*;

#[allow(unused)]
use crate::Pallet as Container;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_core::H256;

benchmarks! {
	set_default_url {
		let s = BoundedVec::try_from("http://127.0.0.1:8000/static".as_bytes().to_vec()).unwrap();
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root, s.clone())
	verify {
		assert_eq!(DefaultUrl::<T>::get(), Some(s));
	}

	register_app {
		let hash = H256::from([1; 32]);
		let file_name = BoundedVec::try_from("test".as_bytes().to_vec()).unwrap();
		let file_size = 123;
		let args = BoundedVec::try_from("--chain dev".as_bytes().to_vec()).unwrap();
		let log = Some(BoundedVec::try_from("aaaa".as_bytes().to_vec()).unwrap());
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller),             hash,
	file_name,
	file_size,
	args,
	log)
	verify {
		let app = APPInfoMap::<T>::get(1).unwrap();
		assert_eq!(app.app_hash, H256::from([1; 32]));
	}
}

impl_benchmark_test_suite!(Container, crate::mock::new_test_ext(), crate::mock::Test,);
