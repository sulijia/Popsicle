#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{BuildGenesisConfig, Randomness};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Hash;
	use sp_std::vec::Vec;
	use super::*;

	pub type RoundIndex = u32;

	/// Pallet for sequencer grouping
	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		type Randomness: Randomness<Self::Hash, BlockNumberFor<Self>>;

		/// Maximum size of each sequencer group
		#[pallet::constant]
		type MaxGroupSize: Get<u32>;

		/// Maximum sequencer group number
		#[pallet::constant]
		type MaxGroupNumber: Get<u32>;
	}

	pub trait SequencerGroup<AccountId, BlockNumber> {
		fn trigger_group(candidates: Vec<AccountId>, starting_block: BlockNumber, round_index: RoundIndex) -> DispatchResult;
		fn account_in_group(account: AccountId) -> Result<u32, DispatchError>;
		fn all_group_ids() -> Vec<u32>;
		fn next_round() -> NextRound<BlockNumber>;
	}

	#[derive(Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen, Default)]
	pub struct NextRound<BlockNumber> {
		pub starting_block: BlockNumber,
		pub round_index: RoundIndex,
	}

	#[pallet::storage]
	#[pallet::getter(fn group_members)]
	pub type GroupMembers<T: Config> = StorageValue<_, BoundedVec<BoundedVec<T::AccountId, T::MaxGroupSize>, T::MaxGroupNumber>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_round)]
	pub type NextRoundStorage<T: Config> = StorageValue<_, NextRound<BlockNumberFor<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn max_group_size)]
	pub(super) type GroupSize<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn max_group_number)]
	pub(super) type GroupNumber<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub group_size: u32,
		pub group_number: u32,
		_marker: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				group_size: 2u32,
				group_number: 3u32,
				_marker: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			GroupSize::<T>::put(&self.group_size);
			GroupNumber::<T>::put(&self.group_number);
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		CandidatesNotEnough,
		GroupSizeTooLarge,
		GroupNumberTooLarge,
		AccountNotInGroup,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Updated the sequencer group.
		SequencerGroupUpdated {
			starting_block: BlockNumberFor<T>,
			round_index: u32,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_group_metric())]
		pub fn set_group_metric(origin: OriginFor<T>, group_size: u32, group_number: u32) -> DispatchResult {
			ensure_root(origin)?;
			// check if group_size is no more than MaxGroupSize
			ensure!(group_size <= T::MaxGroupSize::get(), Error::<T>::GroupSizeTooLarge);
			// check if group_number is no more than MaxGroupNumber
			ensure!(group_number <= T::MaxGroupNumber::get(), Error::<T>::GroupNumberTooLarge);
			GroupSize::<T>::put(group_size);
			GroupNumber::<T>::put(group_number);
			Ok(())
		}

		#[cfg(feature = "runtime-benchmarks")]
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::benchmark_trigger_group(T::MaxGroupSize::get(), T::MaxGroupNumber::get()))]
		pub fn benchmark_trigger_group(origin: OriginFor<T>, candidates: Vec<T::AccountId>, starting_block: BlockNumberFor<T>, round_index: RoundIndex) -> DispatchResult {
			ensure_root(origin)?;
			let _ = <Self as SequencerGroup<T::AccountId, BlockNumberFor<T>>>::trigger_group(candidates, starting_block, round_index);
			Ok(())
		}
	}

	pub struct SimpleRandomness<T>(PhantomData<T>);

	impl<T: Config> Randomness<T::Hash, BlockNumberFor<T>> for SimpleRandomness<T> {
		fn random(subject: &[u8]) -> (T::Hash, BlockNumberFor<T>) {
			let hash = T::Hashing::hash(subject);
			let current_block = frame_system::Pallet::<T>::block_number();
			(hash, current_block)
		}

		fn random_seed() -> (T::Hash, BlockNumberFor<T>) {
			Self::random(b"seed")
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn shuffle_accounts(mut accounts: Vec<T::AccountId>) -> Vec<T::AccountId> {
			// let random_seed = Self::get_and_increment_nonce();
			let random_seed = frame_system::Pallet::<T>::parent_hash().encode();
			let random_value = T::Randomness::random(&random_seed);
			let random_value = <u64>::decode(&mut random_value.0.as_ref()).unwrap_or(0);

			for i in (1..accounts.len()).rev() {
				let j: usize = (random_value as usize) % (i + 1);
				accounts.swap(i, j);
			}

			accounts
		}
    }

	impl<T: Config> SequencerGroup<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
		fn trigger_group(candidates: Vec<T::AccountId>, starting_block: BlockNumberFor<T>, round_index: RoundIndex) -> DispatchResult {
			// check if the length of candidates is enough to form groups required
			let group_size = GroupSize::<T>::get();
			let group_number = GroupNumber::<T>::get();
			ensure!(candidates.len() >= (group_size * group_number) as usize, Error::<T>::CandidatesNotEnough);

			// shuffle the candidate list and split the candidates into groups
			// and store the groups into storage
			// and emit the event
			let mut groups: BoundedVec<BoundedVec<T::AccountId, T::MaxGroupSize>, T::MaxGroupNumber> = BoundedVec::new();
			let mut candidates = Pallet::<T>::shuffle_accounts(candidates);
			for _ in 0..group_number {
				let mut group: BoundedVec<T::AccountId, T::MaxGroupSize> = BoundedVec::new();
				for _ in 0..group_size {
					group.try_push(candidates.pop().unwrap()).expect("can't reach here");
				}
				groups.try_push(group).expect("can't reach here");
			}
			GroupMembers::<T>::put(&groups);

			NextRoundStorage::<T>::put(NextRound {
				starting_block,
				round_index,
			});
			Self::deposit_event(Event::SequencerGroupUpdated {
				starting_block,
				round_index,
			});
			Ok(())
		}

		fn account_in_group(account: T::AccountId) -> Result<u32, DispatchError> {
			let groups = GroupMembers::<T>::get();
			for (index, group) in groups.iter().enumerate() {
				if group.contains(&account) {
					return Ok(index as u32);
				}
			}
			Err(Error::<T>::AccountNotInGroup.into())
		}

		fn all_group_ids() -> Vec<u32> {
			let group_count = GroupMembers::<T>::get().len();
			(0..group_count as u32).collect()
		}

		fn next_round() -> NextRound<BlockNumberFor<T>> {
			NextRoundStorage::<T>::get()
		}
	}
}

