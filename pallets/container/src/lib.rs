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
use derivative::Derivative;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use pallet_sequencer_grouping::SequencerGroup;
use primitives_container::{DownloadInfo, ProcessorDownloadInfo};
use scale_info::{prelude::vec::Vec, TypeInfo};
use sp_runtime::BoundedVec;
use sp_std::{boxed::Box, vec};
pub use weights::*;

#[derive(Derivative, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[derivative(
	Clone(bound = ""),
	Eq(bound = ""),
	PartialEq(bound = ""),
	Debug(bound = ""),
	Default(bound = "")
)]
#[codec(encode_bound())]
#[codec(decode_bound())]
#[scale_info(bounds(), skip_type_params(T))]
pub struct AppClient<T: Config> {
	pub app_hash: Hash,

	pub file_name: BoundedVec<u8, T::MaxLengthFileName>,

	pub size: u32,

	pub args: Option<BoundedVec<u8, T::MaxArgLength>>,

	pub log: Option<BoundedVec<u8, T::MaxLengthFileName>>,

	pub is_docker_image: Option<bool>,

	pub docker_image: Option<BoundedVec<u8, T::MaxLengthFileName>>,
}

#[derive(Encode, Decode, Default, Clone, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct APPInfo<T: Config> {
	creator: T::AccountId,

	project_name: BoundedVec<u8, T::MaxLengthFileName>,

	consensus_client: AppClient<T>,

	batch_client: AppClient<T>,
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
			creator: T::AccountId,
			appid: u32,
			project_name: BoundedVec<u8, T::MaxLengthFileName>,
			consensus_client: BoundedVec<u8, T::MaxLengthFileName>,
			consensus_hash: Hash,
			consensus_size: u32,
			batch_client: BoundedVec<u8, T::MaxLengthFileName>,
			batch_hash: Hash,
			batch_size: u32,
		},
		SetDownloadURL {
			url: BoundedVec<u8, T::MaxUrlLength>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		AppNotExist,
		AccountInconsistent,
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
					Some(_app_id) => {
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
			project_name: BoundedVec<u8, T::MaxLengthFileName>,
			consensus_client: Box<AppClient<T>>,
			batch_client: Box<AppClient<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let old_application_id = NextApplicationID::<T>::get();
			let consensus_app = *consensus_client;
			let batch_app = *batch_client;
			APPInfoMap::<T>::insert(
				old_application_id,
				APPInfo {
					creator: who.clone(),
					project_name: project_name.clone(),
					consensus_client: consensus_app.clone(),
					batch_client: batch_app.clone(),
				},
			);

			NextApplicationID::<T>::set(old_application_id + 1);

			let mut inuse_apps = InuseMap::<T>::get();
			inuse_apps.try_push(false).map_err(|_| Error::<T>::AppNotExist)?;

			InuseMap::<T>::put(inuse_apps);

			Pallet::<T>::deposit_event(Event::<T>::ReisterApp {
				creator: who,
				appid: old_application_id,
				project_name,
				consensus_client: consensus_app.file_name,
				consensus_hash: consensus_app.app_hash,
				consensus_size: consensus_app.size,
				batch_client: batch_app.file_name,
				batch_hash: batch_app.app_hash,
				batch_size: batch_app.size,
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
		// log::info!("============author:{:?}", author.encode());
		//Get the group ID of the sequencer, error when got 0xFFFFFFFF
		let group_id = Self::get_group_id(author);

		let app_id = GroupAPPMap::<T>::get(group_id)?;

		let app_info = APPInfoMap::<T>::get(app_id).ok_or(Error::<T>::AppNotExist).ok()?;

		let url = DefaultUrl::<T>::get()?;

		let consensus_client = app_info.consensus_client;

		let args = consensus_client.args.and_then(|log| Some(log.as_slice().to_vec()));

		let log = consensus_client.log.and_then(|log| Some(log.as_slice().to_vec()));

		let is_docker_image =
			if let Some(is_docker) = consensus_client.is_docker_image { is_docker } else { false };

		let docker_image = consensus_client
			.docker_image
			.and_then(|docker_image| Some(docker_image.as_slice().to_vec()));

		Some(DownloadInfo {
			app_id,
			app_hash: consensus_client.app_hash,
			file_name: consensus_client.file_name.into(),
			size: consensus_client.size,
			group: group_id,
			url: url.into(),
			args,
			log,
			is_docker_image,
			docker_image,
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
		if let Ok(group_id) = group_id_result {
			log::info!("new groupID:{:?}", group_id);
			group_id
		} else {
			0xFFFFFFFF
		}
	}
	pub fn get_groups() -> Vec<u32> {
		<pallet_sequencer_grouping::Pallet<T>>::all_group_ids()
	}

	pub fn processor_run(author: T::AccountId) -> Vec<ProcessorDownloadInfo> {
		let processors = vec![1, 2];
		let mut download_infos: Vec<ProcessorDownloadInfo> = Vec::new();
		if Self::get_groups().len() == 0 {
			return download_infos;
		}
		let url = DefaultUrl::<T>::get().expect("Need set url");

		for app_id in processors {
			let p_app_info = APPInfoMap::<T>::get(app_id);

			if let Some(app_info) = p_app_info {
				let batch_client = app_info.batch_client;

				let args = batch_client.args.and_then(|log| Some(log.as_slice().to_vec()));

				let log = batch_client.log.and_then(|log| Some(log.as_slice().to_vec()));

				let is_docker_image = if let Some(is_docker) = batch_client.is_docker_image {
					is_docker
				} else {
					false
				};

				let docker_image = batch_client
					.docker_image
					.and_then(|docker_image| Some(docker_image.as_slice().to_vec()));
				download_infos.push(ProcessorDownloadInfo {
					app_hash: batch_client.app_hash,
					file_name: batch_client.file_name.into(),
					size: batch_client.size,
					url: url.clone().into(),
					args,
					log,
					is_docker_image,
					docker_image,
				});
			}
		}
		download_infos
	}
}
