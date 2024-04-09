#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
use codec::{Decode, Encode, MaxEncodedLen};
use cumulus_primitives_core::relay_chain::Hash;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use pallet_sequencer_grouping::SequencerGroup;
use primitives_container::DownloadInfo;
use scale_info::{prelude::vec::Vec, TypeInfo};
use sp_runtime::BoundedVec;
use sp_std::vec;
pub use weights::*;

#[derive(Encode, Decode, Default, Clone, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct APPInfo<T: Config> {
	app_hash: Hash,
	creator: T::AccountId,
	file_name: BoundedVec<u8, T::MaxLengthFileName>,
	uploaded: bool,
	size: u32,
	args: Option<BoundedVec<u8, T::MaxArgLength>>,
	log: Option<BoundedVec<u8, T::MaxLengthFileName>>,
}
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_sequencer_grouping::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type MaxLengthFileName: Get<u32>;

		#[pallet::constant]
		type MaxRuningAPP: Get<u32>;

		#[pallet::constant]
		type MaxUrlLength: Get<u32>;

		#[pallet::constant]
		type MaxArgCount: Get<u32>;

		#[pallet::constant]
		type MaxArgLength: Get<u32>;
	}

	#[pallet::type_value]
	pub fn ApplicationIDOnEmpty<T: Config>() -> u32 {
		1
	}
	#[pallet::storage]
	#[pallet::getter(fn next_application_id)]
	pub type NextApplicationID<T> = StorageValue<_, u32, ValueQuery, ApplicationIDOnEmpty<T>>;

	#[pallet::storage]
	#[pallet::getter(fn default_url)]
	pub type DefaultUrl<T: Config> = StorageValue<_, BoundedVec<u8, T::MaxUrlLength>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn appinfo_map)]
	pub type APPInfoMap<T: Config> = StorageMap<_, Twox64Concat, u32, APPInfo<T>, OptionQuery>;

	// app_id,inuse
	#[pallet::storage]
	#[pallet::getter(fn inuse_map)]
	pub type InuseMap<T: Config> = StorageValue<_, BoundedVec<bool, T::MaxRuningAPP>, ValueQuery>;

	// groupid,app_id
	#[pallet::storage]
	#[pallet::getter(fn group_app_map)]
	pub type GroupAPPMap<T: Config> = StorageMap<_, Twox64Concat, u32, u32, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ReisterApp {
			appid: u32,
			file_name: BoundedVec<u8, T::MaxLengthFileName>,
			hash: Hash,
			size: u32,
		},
		SetDownloadURL {
			url: BoundedVec<u8, T::MaxUrlLength>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		AppNotExist,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_: BlockNumberFor<T>) {
			let groups = Self::get_groups();
			log::info!("groups:{:?}", groups);

			let mut inuse_apps = InuseMap::<T>::get();
			log::info!("inuse_apps:{:?}", inuse_apps);

			for group in groups.iter() {
				let app = GroupAPPMap::<T>::get(group);
				match app {
					Some(app_id) => {
						// TODO:alloced app to group,do nothing??
						// GroupAPPMap::<T>::mutate(group, |id| *id=Some((index + 1) as u64));
					},
					None => {
						// alloc app to group
						let alloc_apps = inuse_apps.len();

						let mut index = 0;

						while index < alloc_apps {
							if !inuse_apps[index] {
								inuse_apps[index] = true;

								InuseMap::<T>::mutate(|inuses| inuses[index] = true);

								GroupAPPMap::<T>::insert(group, (index + 1) as u32);

								break;
							}
							index += 1;
						}
						if index == alloc_apps {
							// all is inuse, can not alloc,do nothing,just wait
						}
					},
				}
			}
			log::info!("inuse_apps:{:?}", inuse_apps);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_app())]
		pub fn register_app(
			origin: OriginFor<T>,
			app_hash: Hash,
			file_name: BoundedVec<u8, T::MaxLengthFileName>,
			size: u32,
			args: Option<BoundedVec<u8, T::MaxArgLength>>,
			log: Option<BoundedVec<u8, T::MaxLengthFileName>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let old_application_id = NextApplicationID::<T>::get();

			APPInfoMap::<T>::insert(
				old_application_id,
				APPInfo {
					app_hash,
					creator: who,
					file_name: file_name.clone(),
					uploaded: false,
					size,
					args,
					log,
				},
			);

			NextApplicationID::<T>::set(old_application_id + 1);

			let mut inuse_apps = InuseMap::<T>::get();
			inuse_apps.try_push(false).map_err(|_| Error::<T>::AppNotExist)?;

			InuseMap::<T>::put(inuse_apps);

			Pallet::<T>::deposit_event(Event::<T>::ReisterApp {
				appid: old_application_id,
				file_name,
				hash: app_hash,
				size,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_default_url())]
		pub fn set_default_url(
			origin: OriginFor<T>,
			url: BoundedVec<u8, T::MaxUrlLength>,
		) -> DispatchResult {
			ensure_root(origin)?;

			DefaultUrl::<T>::put(url.clone());

			Pallet::<T>::deposit_event(Event::<T>::SetDownloadURL { url });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	// Obtain application information corresponding to the group.
	// If no group has been assigned or there are no available apps in the group, return None
	pub fn shuld_load(author: T::AccountId) -> Option<DownloadInfo> {
		log::info!("============author:{:?}", author.encode());
		//Get the group ID of the sequencer, error when got 0xFFFFFFFF
		let group_id = Self::get_group_id(author);

		let app_id = GroupAPPMap::<T>::get(group_id)?;

		let app_info = APPInfoMap::<T>::get(app_id).ok_or(Error::<T>::AppNotExist).ok()?;

		let url = DefaultUrl::<T>::get()?;

		let args = app_info.args.and_then(|log| Some(log.as_slice().to_vec()));

		let log = app_info.log.and_then(|log| Some(log.as_slice().to_vec()));

		Some(DownloadInfo {
			app_hash: app_info.app_hash,
			file_name: app_info.file_name.into(),
			size: app_info.size,
			group: group_id,
			url: url.into(),
			args,
			log,
		})
	}

	pub fn should_run() -> bool {
		let next_round = <pallet_sequencer_grouping::Pallet<T>>::next_round();

		let block_number = <frame_system::Pallet<T>>::block_number();

		if next_round.starting_block == block_number {
			true
		} else {
			false
		}
	}

	pub fn get_group_id(author: T::AccountId) -> u32 {
		let group_id_result = <pallet_sequencer_grouping::Pallet<T>>::account_in_group(author);
		log::info!("new groupID:{:?}", group_id_result);

		if group_id_result.is_ok() {
			group_id_result.unwrap()
		} else {
			0xFFFFFFFF
		}
	}
	pub fn get_groups() -> Vec<u32> {
		<pallet_sequencer_grouping::Pallet<T>>::all_group_ids()
	}
}
