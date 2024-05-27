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
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{BuildGenesisConfig, Randomness},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Hash;
	use sp_std::vec::Vec;

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

		/// Maximum length of IP
		#[pallet::constant]
		type MaxLengthIP: Get<u32>;

		/// Maximum number of running app
		#[pallet::constant]
		type MaxRunningAPP: Get<u32>;
	}

	pub trait SequencerGroup<AccountId, BlockNumber> {
		fn total_selected() -> u32;
		fn trigger_group(
			candidates: Vec<AccountId>,
			starting_block: BlockNumber,
			round_index: RoundIndex,
		) -> DispatchResult;
		fn account_in_group(account: AccountId) -> Result<u32, DispatchError>;
		fn all_group_ids() -> Vec<u32>;
		fn next_round() -> NextRound<BlockNumber>;
	}

	#[derive(
		Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen, Default,
	)]
	pub struct NextRound<BlockNumber> {
		pub starting_block: BlockNumber,
		pub round_index: RoundIndex,
	}

	#[pallet::storage]
	#[pallet::getter(fn group_members)]
	pub type GroupMembers<T: Config> = StorageValue<
		_,
		BoundedVec<BoundedVec<T::AccountId, T::MaxGroupSize>, T::MaxGroupNumber>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn next_round)]
	pub type NextRoundStorage<T: Config> =
		StorageValue<_, NextRound<BlockNumberFor<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn max_group_size)]
	pub(super) type GroupSize<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn max_group_number)]
	pub(super) type GroupNumber<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn processor_info)]
	pub(crate) type ProcessorInfo<T: Config> = StorageValue<
		_,
		BoundedVec<
			(T::AccountId, BoundedVec<u8, T::MaxLengthIP>, BoundedVec<u32, T::MaxRunningAPP>),
			T::MaxRunningAPP,
		>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub group_size: u32,
		pub group_number: u32,
		pub _marker: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { group_size: 2u32, group_number: 3u32, _marker: Default::default() }
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
		TooManyProcessors,
		NoProcessors,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Updated the sequencer group.
		SequencerGroupUpdated { starting_block: BlockNumberFor<T>, round_index: u32 },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_group_metric())]
		pub fn set_group_metric(
			origin: OriginFor<T>,
			group_size: u32,
			group_number: u32,
		) -> DispatchResult {
			ensure_root(origin)?;
			// check if group_size is no more than MaxGroupSize
			ensure!(group_size <= T::MaxGroupSize::get(), Error::<T>::GroupSizeTooLarge);
			// check if group_number is no more than MaxGroupNumber
			ensure!(group_number <= T::MaxGroupNumber::get(), Error::<T>::GroupNumberTooLarge);
			GroupSize::<T>::put(group_size);
			GroupNumber::<T>::put(group_number);
			Ok(())
		}

		//#[cfg(feature = "runtime-benchmarks")]
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::benchmark_trigger_group(
			T::MaxGroupSize::get(),
			T::MaxGroupNumber::get()
		))]
		pub fn benchmark_trigger_group(
			origin: OriginFor<T>,
			candidates: Vec<T::AccountId>,
			starting_block: BlockNumberFor<T>,
			round_index: RoundIndex,
		) -> DispatchResult {
			ensure_root(origin)?;
			let _ = <Self as SequencerGroup<T::AccountId, BlockNumberFor<T>>>::trigger_group(
				candidates,
				starting_block,
				round_index,
			);
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::register_processor())]
		pub fn register_processor(
			origin: OriginFor<T>,
			ip_address: BoundedVec<u8, T::MaxLengthIP>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut processor_info = crate::pallet::ProcessorInfo::<T>::get();

			if let Some(existing) =
				processor_info.iter_mut().find(|(account, _, _)| account == &who)
			{
				if existing.1 == ip_address {
					return Ok(());
				}
				existing.1 = ip_address;
			} else {
				let processor = (who.clone(), ip_address, BoundedVec::new());
				processor_info
					.try_push(processor)
					.map_err(|_| crate::pallet::Error::<T>::TooManyProcessors)?;
			}

			crate::pallet::ProcessorInfo::<T>::put(&processor_info);
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
			// let random_value = T::Randomness::random(&random_seed);
			// let random_value = <u64>::decode(&mut random_value.0.as_ref()).unwrap_or(0);
			let random_value = random_seed[0];
			for i in (1..accounts.len()).rev() {
				let j: usize = (random_value as usize) % (i + 1);
				accounts.swap(i, j);
			}

			accounts
		}

		pub fn assign_processors_to_groups(group_number: u32) -> DispatchResult {
			let mut processor_info = ProcessorInfo::<T>::get();
			let processor_count = processor_info.len() as u32;

			// 如果 processor_info 为空，则直接返回
			if processor_count == 0 {
				return Err(Error::<T>::NoProcessors.into());
			}

			for (_, _, groups) in &mut processor_info {
				groups.clear();
			}

			for group_id in 0..=group_number - 1 {
				let processor_index = group_id % processor_count;
				let processor = &mut processor_info[processor_index as usize];

				if !processor.2.contains(&group_id) {
					processor.2.try_push(group_id).map_err(|_| Error::<T>::TooManyProcessors)?;
				}
			}

			ProcessorInfo::<T>::put(processor_info);
			Ok(())
		}

		pub fn get_group_ids(account: T::AccountId) -> Vec<u32> {
			let processor_info = ProcessorInfo::<T>::get();
			if let Some((_, _, group_ids)) =
				processor_info.iter().find(|(acc, _, _)| *acc == account)
			{
				group_ids.clone().into_inner()
			} else {
				Vec::new()
			}
		}
	}

	impl<T: Config> SequencerGroup<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
		fn total_selected() -> u32 {
			GroupSize::<T>::get() * GroupNumber::<T>::get()
		}
		fn trigger_group(
			candidates: Vec<T::AccountId>,
			starting_block: BlockNumberFor<T>,
			round_index: RoundIndex,
		) -> DispatchResult {
			// check if the length of candidates is enough to form groups required
			let group_size = GroupSize::<T>::get();
			let group_number = GroupNumber::<T>::get();
			ensure!(
				candidates.len() >= (group_size * group_number) as usize,
				Error::<T>::CandidatesNotEnough
			);

			// shuffle the candidate list and split the candidates into groups
			// and store the groups into storage
			// and emit the event
			let mut groups: BoundedVec<
				BoundedVec<T::AccountId, T::MaxGroupSize>,
				T::MaxGroupNumber,
			> = BoundedVec::new();
			let mut candidates = Pallet::<T>::shuffle_accounts(candidates);
			for _ in 0..group_number {
				let mut group: BoundedVec<T::AccountId, T::MaxGroupSize> = BoundedVec::new();
				for _ in 0..group_size {
					group.try_push(candidates.pop().unwrap()).expect("can't reach here");
				}
				groups.try_push(group).expect("can't reach here");
			}
			GroupMembers::<T>::put(&groups);

			Pallet::<T>::assign_processors_to_groups(group_number)?;

			NextRoundStorage::<T>::put(NextRound { starting_block, round_index });
			Self::deposit_event(Event::SequencerGroupUpdated { starting_block, round_index });
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
