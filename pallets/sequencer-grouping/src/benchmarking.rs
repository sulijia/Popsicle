//! Benchmarking setup for pallet-sequencer-grouping

use super::*;

#[allow(unused)]
use crate::Pallet as SequencerGrouping;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_core::H256;

benchmarks! {
	set_group_metric {
		let group_size: u32 = 3;
		let group_number: u32 = 5;
	}: _(RawOrigin::Root, group_size, group_number)

	verify {
		assert_eq!(GroupSize::<T>::get(), group_size);
		assert_eq!(GroupNumber::<T>::get(), group_number);
	}
}

impl_benchmark_test_suite!(SequencerGrouping, crate::mock::new_test_ext(), crate::mock::Test,);