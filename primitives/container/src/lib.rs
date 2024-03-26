#![cfg_attr(not(feature = "std"), no_std)]
use parity_scale_codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;
#[derive(Debug, Clone, TypeInfo, Encode, Decode)]
pub struct DownloadInfo {
	pub app_hash: H256,
	pub file_name: Vec<u8>,
	pub size: u32,
	pub group: u32,
	pub url: Vec<u8>,
	pub args: Option<Vec<u8>>,
	pub log: Option<Vec<u8>>,
}

sp_api::decl_runtime_apis! {
	#[api_version(2)]
	pub trait ContainerRuntimeApi<AuthorityId> where
	AuthorityId:Codec
	{
		fn shuld_load(author:AuthorityId)->Option<DownloadInfo>;
		fn should_run()-> bool;
		fn get_group_id(author:AuthorityId) ->u32;
		fn get_groups()->Vec<u32>;
	}
}
