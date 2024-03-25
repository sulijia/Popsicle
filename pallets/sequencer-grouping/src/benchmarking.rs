//! Benchmarking setup for pallet-sequencer-grouping

use super::*;

#[allow(unused)]
use crate::Pallet as SequencerGrouping;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::pallet_prelude::Get;
use frame_system::RawOrigin;
use sp_std::vec::Vec;

benchmarks! {
	set_group_metric {
		let group_size: u32 = 3;
		let group_number: u32 = 5;
	}: _(RawOrigin::Root, group_size, group_number)

	verify {
		assert_eq!(GroupSize::<T>::get(), group_size);
		assert_eq!(GroupNumber::<T>::get(), group_number);
	}

	benchmark_trigger_group {
		let s in 1 .. T::MaxGroupSize::get() as u32;
		let n in 1 .. T::MaxGroupNumber::get() as u32;

        let mut candidates: Vec<T::AccountId> = Vec::new();
        for i in 0..(s * n) {
            let candidate: T::AccountId = account("candidate", i, 0);
            candidates.push(candidate);
        }
        let starting_block = frame_system::Pallet::<T>::block_number();
        let round_index = 1u32;

    }: _(RawOrigin::Root, candidates, starting_block, round_index)
}

impl_benchmark_test_suite!(SequencerGrouping, crate::mock::new_test_ext(), crate::mock::Test,);