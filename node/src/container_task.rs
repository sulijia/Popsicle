use codec::Decode;
use cumulus_primitives_core::{ParaId, PersistedValidationData};
use cumulus_relay_chain_interface::{RelayChainInterface, RelayChainResult};
use futures::{lock::Mutex, pin_mut, select, FutureExt, Stream, StreamExt};
use polkadot_primitives::OccupiedCoreAssumption;
use primitives_container::{ContainerRuntimeApi, DownloadInfo, ProcessorDownloadInfo};
use reqwest::{
	self,
	header::{HeaderValue, CONTENT_LENGTH, RANGE},
	StatusCode,
};
use ring::digest::{Context, Digest, SHA256};
use sc_client_api::UsageProvider;
use sc_service::TaskManager;
use sp_api::ProvideRuntimeApi;
use sp_core::{hexdisplay::HexDisplay, offchain::OffchainStorage, H256};
use sp_keystore::KeystorePtr;
use sp_offchain::STORAGE_PREFIX;
use sp_runtime::{
	traits::{Block as BlockT, Header as HeaderT},
	AccountId32,
};
use std::{
	collections::HashMap,
	error::Error,
	fs::{self, File, Permissions},
	io::{BufReader, Read},
	os::unix::fs::PermissionsExt,
	path::{Path, PathBuf},
	process::{Child, Command, Stdio},
	str::FromStr,
	sync::Arc,
};

pub const RUN_ARGS_KEY: &str = "run_args";
pub const SYNC_ARGS_KEY: &str = "sync_args";
pub const OPTION_ARGS_KEY: &str = "option_args";
pub const P_RUN_ARGS_KEY: &str = "p_run_args";
pub const P_OPTION_ARGS_KEY: &str = "p_option_args";
struct PartialRangeIter {
	start: u64,
	end: u64,
	buffer_size: u32,
}

impl PartialRangeIter {
	pub fn new(start: u64, end: u64, buffer_size: u32) -> Result<Self, Box<dyn Error>> {
		if buffer_size == 0 {
			Err("invalid buffer_size, give a value greater than zero.")?;
		}
		Ok(PartialRangeIter { start, end, buffer_size })
	}
}

impl Iterator for PartialRangeIter {
	type Item = HeaderValue;
	fn next(&mut self) -> Option<Self::Item> {
		if self.start > self.end {
			None
		} else {
			let prev_start = self.start;
			self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
			Some(
				HeaderValue::from_str(&format!("bytes={}-{}", prev_start, self.start - 1))
					.expect("string provided by format!"),
			)
		}
	}
}

async fn sha256_digest<R: Read>(mut reader: R) -> Result<Digest, Box<dyn Error + Send + Sync>> {
	let mut context = Context::new(&SHA256);
	let mut buffer = [0; 1024];

	loop {
		let count = reader.read(&mut buffer)?;
		if count == 0 {
			break;
		}
		context.update(&buffer[..count]);
	}

	Ok(context.finish())
}

async fn download_sdk(
	data_path: PathBuf,
	file_name: &str,
	file_hash: H256,
	url: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	//firt create dir
	let path_str = format!("{}/sdk", data_path.as_os_str().to_str().ok_or("path error!")?);

	let download_dir = Path::new(&path_str);
	if !download_dir.exists() {
		fs::create_dir(&download_dir)?
	}

	const CHUNK_SIZE: u32 = 1024000; // 1 M

	let client = reqwest::blocking::Client::new();

	let web_path = format!("{}/{}", url, file_name);
	log::info!("=============download:{:?}", web_path);

	let response = client.head(&web_path).send()?;

	let length = response
		.headers()
		.get(CONTENT_LENGTH)
		.ok_or("response doesn't include the content length")?;

	let length = u64::from_str(length.to_str()?).map_err(|_| "invalid Content-Length header")?;
	log::info!("==========total length:{:?}", length);

	let download_path = format!("{}/{}", path_str, file_name);
	log::info!("=============download_path:{:?}", download_path);

	let download_dir = Path::new(&download_path);

	let mut output_file = File::create(download_dir)?;

	println!("starting download...");
	for range in PartialRangeIter::new(0, length - 1, CHUNK_SIZE).unwrap() {
		println!("range {:?}", range);
		let mut response = client.get(&web_path).header(RANGE, range).send()?;

		let status = response.status();
		if !(status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT) {
			log::info!("Unexpected server response: {:?}", status);
			break;
		}
		std::io::copy(&mut response, &mut output_file)?;
	}

	let content = response.text()?;
	std::io::copy(&mut content.as_bytes(), &mut output_file)?;

	println!("Finished with success!");

	output_file.set_permissions(Permissions::from_mode(0o777))?;
	//check file hash
	println!("check file hash");
	let input = File::open(download_dir)?;
	let reader = BufReader::new(input);
	let digest = sha256_digest(reader).await?;

	println!("SHA-256 digest is {:?}", digest);
	if digest.as_ref() == file_hash.as_bytes() {
		println!("check ok");
	} else {
		println!("check fail");
		Err("Hash check fail.")?;
	}

	Ok(())
}
async fn need_download(
	data_path: &str,
	app_hash: H256,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
	let input =
		if let Ok(open_file) = File::open(data_path) { open_file } else { return Ok(true) };

	let reader = BufReader::new(input);

	let digest = sha256_digest(reader).await?;

	println!("SHA-256 digest is {:?}", digest);
	if digest.as_ref() == app_hash.as_bytes() {
		Ok(false)
	} else {
		Ok(true)
	}
}

async fn check_docker_image_exist(
	docker_image: &str,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
	let ls_cmd = format!("image ls {}", docker_image);
	let args: Vec<&str> = ls_cmd.split(' ').into_iter().map(|arg| arg).collect();
	let mut instance = Command::new("docker").stdout(Stdio::piped()).args(args).spawn()?;
	let mut result = false;
	if let Some(ls_output) = instance.stdout.take() {
		let grep_cmd = Command::new("grep")
			.arg(docker_image)
			.stdin(ls_output)
			.stdout(Stdio::piped())
			.spawn()?;

		let grep_stdout = grep_cmd.wait_with_output()?;
		instance.wait()?;
		let grep_out = String::from_utf8(grep_stdout.stdout)?;
		if grep_out.len() > 0 {
			result = true;
		}
	}
	Ok(result)
}

async fn download_docker_image(docker_image: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
	let pull_cmd = format!("pull {}", docker_image);
	let args: Vec<&str> = pull_cmd.split(' ').into_iter().map(|arg| arg).collect();
	let mut instance = Command::new("docker").args(args).spawn()?;
	instance.wait()?;
	Ok(())
}

async fn remove_docker_container(container_name: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
	let mut docker_cmd = format!("container stop {}", container_name);
	let mut args: Vec<&str> = docker_cmd.split(' ').into_iter().map(|arg| arg).collect();
	let mut instance = Command::new("docker").args(args).spawn()?;
	instance.wait()?;

	docker_cmd = format!("container remove {}", container_name);
	args = docker_cmd.split(' ').into_iter().map(|arg| arg).collect();
	instance = Command::new("docker").args(args).spawn()?;
	instance.wait()?;
	Ok(())
}

async fn start_docker_container(
	container_name: &str,
	docker_image: &str,
	in_args: Vec<&str>,
	option_args: Option<Vec<u8>>,
	log_file: File,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let docker_cmd = if let Some(option_arg) = option_args {
		let option = std::str::from_utf8(&option_arg)?;
		format!("run -itd --name {} {} {}", container_name, option, docker_image)
	} else {
		format!("run -itd --name {} {}", container_name, docker_image)
	};
	let mut args: Vec<&str> = docker_cmd.split(' ').into_iter().map(|arg| arg).collect();
	args.extend(in_args);
	log::info!("=======================args:{:?}", args);
	let mut instance = Command::new("docker").args(args).stdout(Stdio::from(log_file)).spawn()?;
	instance.wait()?;
	Ok(())
}
#[derive(Debug, PartialEq, Eq)]
enum StartType {
	SYNC,
	RUN,
}
#[derive(Debug, PartialEq, Eq)]
enum InstanceIndex {
	Instance1,
	Instance2,
}
#[derive(Debug, PartialEq, Eq)]
enum RunStatus {
	Pending,
	Downloading,
	Downloaded,
	Running,
}
#[derive(Debug)]
struct RunningApp {
	group_id: u32,
	app_id: u32,
	app_hash: H256,
	running: RunStatus,
	app_info: Option<DownloadInfo>,
	instance1: Option<Child>,
	instance2: Option<Child>,
	instance1_docker: bool,
	instance2_docker: bool,
	instance1_docker_name: Option<Vec<u8>>,
	instance2_docker_name: Option<Vec<u8>>,
	cur_ins: InstanceIndex,
}

#[derive(Debug)]
struct ProcessorInstance {
	app_hash: H256,
	running: RunStatus,
	processor_info: Option<ProcessorDownloadInfo>,
	instance: Option<Child>,
	instance_docker: bool,
	instance_docker_name: Option<Vec<u8>>,
}
#[derive(Debug)]
struct RunningProcessor {
	processors: HashMap<H256, ProcessorInstance>,
}

async fn app_download_task(
	data_path: PathBuf,
	app_info: DownloadInfo,
	running_app: Arc<Mutex<RunningApp>>,
	new_group: u32,
	sync_args: Option<Vec<u8>>,
	option_args: Option<Vec<u8>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let run_as_docker = app_info.is_docker_image;
	let mut start_flag = false;

	if run_as_docker {
		log::info!("===========Download app from docker hub and run the application as a container=========");
		if let Some(image) = app_info.clone().docker_image {
			let docker_image = std::str::from_utf8(&image)?;
			let mut exist_docker_image = check_docker_image_exist(docker_image).await?;
			if !exist_docker_image {
				download_docker_image(docker_image).await?;
				exist_docker_image = check_docker_image_exist(docker_image).await?;
			}
			if exist_docker_image {
				//Start docker container
				start_flag = true;
			}
		}
	} else {
		log::info!(
			"===========Download app from the web and run the application as a process========="
		);
		let url = std::str::from_utf8(&app_info.url)?;

		let download_path = format!(
			"{}/sdk/{}",
			data_path.as_os_str().to_str().ok_or("invalid data_path")?,
			std::str::from_utf8(&app_info.file_name)?
		);

		let need_download = need_download(&download_path, app_info.app_hash).await;
		log::info!("need_download:{:?}", need_download);

		if let Ok(need_down) = need_download {
			if need_down {
				let file_name = std::str::from_utf8(&app_info.file_name)?;
				let result =
					download_sdk(data_path.clone(), file_name, app_info.app_hash, url).await;

				if result.is_ok() {
					start_flag = true;
				} else {
					log::info!("download sdk error:{:?}", result);
				}
			} else {
				start_flag = true;
			}
		}
	}
	if start_flag {
		log::info!("===============start app for sync=================");
		let result = app_run_task(
			data_path,
			app_info.clone(),
			sync_args,
			option_args,
			running_app.clone(),
			StartType::SYNC,
		)
		.await;
		log::info!("start result:{:?}", result);
		let mut app = running_app.lock().await;
		app.app_info = Some(app_info);
		app.group_id = new_group;
		app.running = RunStatus::Downloaded;
	} else {
		let mut app = running_app.lock().await;
		app.running = RunStatus::Pending;
	}
	Ok(())
}

async fn processor_run_task(
	data_path: PathBuf,
	processor_info: ProcessorDownloadInfo,
	run_args: Option<Vec<u8>>,
	option_args: Option<Vec<u8>>,
	running_processor: Arc<Mutex<RunningProcessor>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let run_as_docker = processor_info.is_docker_image;

	let extension_args_clone;

	let extension_args: Option<Vec<&str>> = if let Some(run_args) = run_args {
		extension_args_clone = run_args.clone();

		Some(
			std::str::from_utf8(&extension_args_clone)?
				.split(' ')
				.into_iter()
				.map(|arg| std::str::from_utf8(arg.as_bytes()).unwrap())
				.collect(),
		)
	} else {
		None
	};

	let app_args_clone;

	let mut args: Vec<&str> = if let Some(app_args) = processor_info.args {
		app_args_clone = app_args.clone();

		std::str::from_utf8(&app_args_clone)?
			.split(' ')
			.into_iter()
			.map(|arg| std::str::from_utf8(arg.as_bytes()).unwrap())
			.collect()
	} else {
		Vec::new()
	};

	if let Some(extension_args) = extension_args {
		args.extend(extension_args);
	}

	let log_file_name;

	let log_file_buf;

	if processor_info.log.is_none() {
		log_file_name = std::str::from_utf8(&processor_info.file_name)?;
	} else {
		log_file_buf = processor_info.log.unwrap();

		log_file_name = std::str::from_utf8(&log_file_buf)?;
	}

	log::info!("log_file_name:{:?}", log_file_name);
	log::info!("args:{:?}", args);
	let outputs = File::create(log_file_name)?;

	let errors = outputs.try_clone()?;

	// start new instance
	let mut instance = None;
	if run_as_docker {
		let image_name = processor_info.docker_image.ok_or("docker image not exist")?;
		let docker_image = std::str::from_utf8(&image_name)?;
		let start_result = start_docker_container(
			std::str::from_utf8(&processor_info.file_name)?,
			docker_image,
			args,
			option_args,
			outputs.try_clone()?,
		)
		.await;
		log::info!("start docker container :{:?}", start_result);
	} else {
		let download_path = format!(
			"{}/sdk/{}",
			data_path.as_os_str().to_str().unwrap(),
			std::str::from_utf8(&processor_info.file_name)?
		);
		instance = Some(
			Command::new(download_path)
				.stdin(Stdio::piped())
				.stderr(Stdio::from(outputs))
				.stdout(Stdio::from(errors))
				.args(args)
				.spawn()
				.expect("failed to execute process"),
		);
	}
	let mut running_processors = running_processor.lock().await;
	let processor_instances = &mut running_processors.processors;
	processor_instances.entry(processor_info.app_hash).and_modify(|app| {
		app.instance = instance;
		if run_as_docker {
			app.instance_docker = true;
			app.instance_docker_name = Some(processor_info.file_name);
		} else {
			app.instance_docker = false;
			app.instance_docker_name = None;
		}
	});

	log::info!("app:{:?}", running_processors);
	Ok(())
}
async fn processor_task(
	data_path: PathBuf,
	processor_info: ProcessorDownloadInfo,
	running_processor: Arc<Mutex<RunningProcessor>>,
	run_args: Option<Vec<u8>>,
	option_args: Option<Vec<u8>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let run_as_docker = processor_info.is_docker_image;
	let mut start_flag = false;

	if run_as_docker {
		log::info!("===========Download app from docker hub and run the application as a container=========");
		if let Some(image) = processor_info.clone().docker_image {
			let docker_image = std::str::from_utf8(&image)?;
			let mut exist_docker_image = check_docker_image_exist(docker_image).await?;
			if !exist_docker_image {
				download_docker_image(docker_image).await?;
				exist_docker_image = check_docker_image_exist(docker_image).await?;
			}
			if exist_docker_image {
				//Start docker container
				start_flag = true;
			}
		}
	} else {
		log::info!(
			"===========Download app from the web and run the application as a process========="
		);
		let url = std::str::from_utf8(&processor_info.url)?;

		let download_path = format!(
			"{}/sdk/{}",
			data_path.as_os_str().to_str().ok_or("invalid data_path")?,
			std::str::from_utf8(&processor_info.file_name)?
		);

		let need_download = need_download(&download_path, processor_info.app_hash).await;
		log::info!("need_download:{:?}", need_download);

		if let Ok(need_down) = need_download {
			if need_down {
				let file_name = std::str::from_utf8(&processor_info.file_name)?;
				let result =
					download_sdk(data_path.clone(), file_name, processor_info.app_hash, url).await;

				if result.is_ok() {
					start_flag = true;
				} else {
					log::info!("download processor client error:{:?}", result);
				}
			} else {
				start_flag = true;
			}
		}
	}
	let status = if start_flag {
		log::info!("===============run processor=================");
		let result = processor_run_task(
			data_path,
			processor_info.clone(),
			run_args,
			option_args,
			running_processor.clone(),
		)
		.await;
		log::info!("start processor result:{:?}", result);
		RunStatus::Downloaded
	} else {
		RunStatus::Pending
	};
	let mut running_processors = running_processor.lock().await;
	let processor_instances = &mut running_processors.processors;
	processor_instances
		.entry(processor_info.app_hash)
		.and_modify(|instance| instance.running = status);
	Ok(())
}

async fn app_run_task(
	data_path: PathBuf,
	app_info: DownloadInfo,
	run_args: Option<Vec<u8>>,
	option_args: Option<Vec<u8>>,
	running_app: Arc<Mutex<RunningApp>>,
	start_type: StartType,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let run_as_docker = app_info.is_docker_image;

	let extension_args_clone;

	let extension_args: Option<Vec<&str>> = if let Some(run_args) = run_args {
		extension_args_clone = run_args.clone();

		Some(
			std::str::from_utf8(&extension_args_clone)?
				.split(' ')
				.into_iter()
				.map(|arg| std::str::from_utf8(arg.as_bytes()).unwrap())
				.collect(),
		)
	} else {
		None
	};

	let app_args_clone;

	let mut args: Vec<&str> = if let Some(app_args) = app_info.args {
		app_args_clone = app_args.clone();

		std::str::from_utf8(&app_args_clone)?
			.split(' ')
			.into_iter()
			.map(|arg| std::str::from_utf8(arg.as_bytes()).unwrap())
			.collect()
	} else {
		Vec::new()
	};

	if let Some(extension_args) = extension_args {
		args.extend(extension_args);
	}

	let log_file_name;

	let log_file_buf;

	if app_info.log.is_none() {
		log_file_name = std::str::from_utf8(&app_info.file_name)?;
	} else {
		log_file_buf = app_info.log.unwrap();

		log_file_name = std::str::from_utf8(&log_file_buf)?;
	}

	log::info!("log_file_name:{:?}", log_file_name);
	log::info!("args:{:?}", args);
	let outputs = File::create(log_file_name)?;

	let errors = outputs.try_clone()?;

	let mut app = running_app.lock().await;

	let (old_instance, instance_docker, op_docker_name, cur_ins) =
		if app.cur_ins == InstanceIndex::Instance1 {
			let is_docker_instance = app.instance1_docker;
			let top_docker_name = app.instance1_docker_name.clone();
			(&mut app.instance1, is_docker_instance, top_docker_name, 1)
		} else {
			let is_docker_instance = app.instance2_docker;
			let top_docker_name = app.instance2_docker_name.clone();
			(&mut app.instance2, is_docker_instance, top_docker_name, 2)
		};
	// stop old instance
	if instance_docker {
		//reomve docker container
		if let Some(docker_name) = op_docker_name {
			let kill_result = remove_docker_container(std::str::from_utf8(&docker_name)?).await;
			log::info!("kill old docker instance:{:?}", kill_result);
		}
	} else {
		if let Some(ref mut old_instance) = old_instance {
			old_instance.kill()?;
			let kill_result = old_instance.wait()?;
			log::info!("kill old instance:{:?}:{:?}", cur_ins, kill_result);
			match kill_result.code() {
				Some(code) => log::info!("Exited with status code: {code}"),
				None => log::info!("Process terminated by signal"),
			}
		}
	}
	// start new instance
	let mut instance: Option<Child> = None;
	if run_as_docker {
		let image_name = app_info.docker_image.ok_or("docker image not exist")?;
		let docker_image = std::str::from_utf8(&image_name)?;
		let start_result = start_docker_container(
			std::str::from_utf8(&app_info.file_name)?,
			docker_image,
			args,
			option_args,
			outputs.try_clone()?,
		)
		.await;
		log::info!("start docker container :{:?}", start_result);
	} else {
		let download_path = format!(
			"{}/sdk/{}",
			data_path.as_os_str().to_str().unwrap(),
			std::str::from_utf8(&app_info.file_name)?
		);
		instance = Some(
			Command::new(download_path)
				.stdin(Stdio::piped())
				.stderr(Stdio::from(outputs))
				.stdout(Stdio::from(errors))
				.args(args)
				.spawn()
				.expect("failed to execute process"),
		);
	}
	if start_type == StartType::RUN {
		if app.cur_ins == InstanceIndex::Instance1 {
			if app.instance2_docker {
				let docker_name_op = app.instance2_docker_name.clone();
				if let Some(docker_name) = docker_name_op {
					let kill_result =
						remove_docker_container(std::str::from_utf8(&docker_name)?).await;
					log::info!("kill docker instance2:{:?}", kill_result);
				}
				app.instance2_docker_name = None;
				app.instance2_docker = false;
			} else {
				let other_instance = &mut app.instance2;
				if let Some(ref mut other_instance) = other_instance {
					other_instance.kill()?;
					let kill_result = other_instance.wait()?;
					log::info!("kill instance2:{:?}", kill_result);
				}
				app.instance2 = None;
			}
			app.instance1 = instance;
			app.cur_ins = InstanceIndex::Instance2;
		} else {
			if app.instance1_docker {
				let docker_name_op = app.instance1_docker_name.clone();
				if let Some(docker_name) = docker_name_op {
					let kill_result =
						remove_docker_container(std::str::from_utf8(&docker_name)?).await;
					log::info!("kill docker instance1:{:?}", kill_result);
				}
				app.instance1_docker_name = None;
				app.instance1_docker = false;
			} else {
				let other_instance = &mut app.instance1;
				if let Some(ref mut other_instance) = other_instance {
					other_instance.kill()?;
					let kill_result = other_instance.wait()?;
					log::info!("kill instance1:{:?}", kill_result);
				}
				app.instance1 = None;
			}
			app.instance2 = instance;
			app.cur_ins = InstanceIndex::Instance1;
		}
	} else {
		if app.cur_ins == InstanceIndex::Instance1 {
			app.instance1 = instance;
			if run_as_docker {
				app.instance1_docker = true;
				app.instance1_docker_name = Some(app_info.file_name);
			} else {
				app.instance1_docker = false;
				app.instance1_docker_name = None;
			}
		} else {
			app.instance2 = instance;
			if run_as_docker {
				app.instance2_docker = true;
				app.instance2_docker_name = Some(app_info.file_name);
			} else {
				app.instance2_docker = false;
				app.instance2_docker_name = None;
			}
		}
	}
	log::info!("app:{:?}", app);
	Ok(())
}

async fn get_offchain_storage<Block, TBackend>(
	offchain_storage: Option<TBackend::OffchainStorage>,
	args: &[u8],
) -> Option<Vec<u8>>
where
	Block: BlockT,
	TBackend: 'static + sc_client_api::backend::Backend<Block> + Send,
{
	if let Some(storage) = offchain_storage {
		storage.get(&STORAGE_PREFIX, args)
	} else {
		None
	}
}

async fn handle_new_best_parachain_head<P, Block, TBackend>(
	validation_data: PersistedValidationData,
	parachain: &P,
	keystore: KeystorePtr,
	data_path: PathBuf,
	running_app: Arc<Mutex<RunningApp>>,
	running_processor: Arc<Mutex<RunningProcessor>>,
	backend: Arc<TBackend>,
) -> Result<(), Box<dyn Error>>
where
	Block: BlockT,
	P: ProvideRuntimeApi<Block> + UsageProvider<Block>,
	P::Api: ContainerRuntimeApi<Block, AccountId32>,
	<Block::Header as HeaderT>::Number: Into<u32>,
	TBackend: 'static + sc_client_api::backend::Backend<Block> + Send,
{
	let offchain_storage = backend.offchain_storage();

	// Check if there is a download task
	let head = validation_data.clone().parent_head.0;

	let parachain_head = match <<Block as BlockT>::Header>::decode(&mut &head[..]) {
		Ok(header) => header,
		Err(err) => return Err(format!("get parachain head error:{:?}", err).into()),
	};

	let hash = parachain_head.hash();

	let xx = keystore.sr25519_public_keys(sp_application_crypto::key_types::AURA)[0];

	// Processor client process
	let ip_address = get_local_info::get_pc_ipv4();

	log::info!("ip_address:{:?}", ip_address);

	let processor_infos: Vec<ProcessorDownloadInfo> =
		parachain.runtime_api().processor_run(hash, xx.into())?;

	log::info!("processor download info:{:?}", processor_infos);

	{
		let mut running_processors = running_processor.lock().await;
		let processors = &mut running_processors.processors;
		for processor_info in processor_infos {
			let app_hash = processor_info.app_hash;
			let processor = processors.entry(app_hash).or_insert(ProcessorInstance {
				app_hash,
				running: RunStatus::Pending,
				processor_info: Some(processor_info.clone()),
				instance: None,
				instance_docker: false,
				instance_docker_name: None,
			});
			let run_status = &processor.running;

			if *run_status == RunStatus::Pending {
				processor.running = RunStatus::Downloading;
				let app_hash = processor.app_hash;
				let p_run_args_key =
					format!("{}:{}", P_RUN_ARGS_KEY, HexDisplay::from(&app_hash.as_bytes()));
				let p_option_args_key =
					format!("{}:{}", P_OPTION_ARGS_KEY, HexDisplay::from(&app_hash.as_bytes()));
				let run_args = get_offchain_storage::<_, TBackend>(
					offchain_storage.clone(),
					p_run_args_key.as_bytes(),
				)
				.await;
				let option_args = get_offchain_storage::<_, TBackend>(
					offchain_storage.clone(),
					p_option_args_key.as_bytes(),
				)
				.await;
				tokio::spawn(processor_task(
					data_path.clone(),
					processor_info,
					running_processor.clone(),
					run_args,
					option_args,
				));
			}
		}
	}
	//Layer2 client process
	let should_load: Option<DownloadInfo> = parachain.runtime_api().shuld_load(hash, xx.into())?;
	log::info!("app download info of sequencer's group:{:?}", should_load);

	{
		let mut app = running_app.lock().await;

		let old_group_id = app.group_id;

		match should_load {
			Some(app_info) => {
				let new_group = app_info.group;
				let app_id = app_info.app_id;
				let app_hash = app_info.app_hash;
				let run_status = &app.running;
				if old_group_id != new_group && *run_status == RunStatus::Pending {
					let sync_args_key =
						format!("{}:{}", SYNC_ARGS_KEY, HexDisplay::from(&app_hash.as_bytes()));

					let option_args_key =
						format!("{}:{}", OPTION_ARGS_KEY, HexDisplay::from(&app_hash.as_bytes()));

					let sync_args = get_offchain_storage::<_, TBackend>(
						offchain_storage.clone(),
						sync_args_key.as_bytes(),
					)
					.await;

					let option_args = get_offchain_storage::<_, TBackend>(
						offchain_storage.clone(),
						option_args_key.as_bytes(),
					)
					.await;

					log::info!("offchain_storage of sync_args:{:?}", sync_args);

					log::info!("offchain_storage of option_args:{:?}", option_args);

					app.running = RunStatus::Downloading;

					app.app_id = app_id;
					app.app_hash = app_hash;
					tokio::spawn(app_download_task(
						data_path.clone(),
						app_info,
						running_app.clone(),
						new_group,
						sync_args,
						option_args.clone(),
					));
				}
			},
			None => log::info!("None"),
		}
	}
	let should_run = parachain.runtime_api().should_run(hash)?;

	if should_run {
		let mut app = running_app.lock().await;
		let run_status = &app.running;
		let app_hash = app.app_hash;
		if let Some(app_info) = app.app_info.clone() {
			if *run_status == RunStatus::Downloaded {
				log::info!("run:{:?}", app);
				let run_args_key =
					format!("{}:{}", RUN_ARGS_KEY, HexDisplay::from(&app_hash.as_bytes()));

				let option_args_key =
					format!("{}:{}", OPTION_ARGS_KEY, HexDisplay::from(&app_hash.as_bytes()));

				let run_args = get_offchain_storage::<_, TBackend>(
					offchain_storage.clone(),
					run_args_key.as_bytes(),
				)
				.await;

				let option_args = get_offchain_storage::<_, TBackend>(
					offchain_storage.clone(),
					option_args_key.as_bytes(),
				)
				.await;
				log::info!("offchain_storage of run_args:{:?}", run_args);
				log::info!("offchain_storage of option_args:{:?}", option_args);
				tokio::spawn(app_run_task(
					data_path,
					app_info,
					run_args,
					option_args,
					running_app.clone(),
					StartType::RUN,
				));

				app.running = RunStatus::Pending;
			}
		}
	}
	Ok(())
}

async fn new_best_heads(
	relay_chain: impl RelayChainInterface + Clone,
	para_id: ParaId,
) -> RelayChainResult<impl Stream<Item = (u32, PersistedValidationData, H256)>> {
	let new_best_notification_stream =
		relay_chain.new_best_notification_stream().await?.filter_map(move |n| {
			let relay_chain = relay_chain.clone();
			async move {
				let relay_head: PersistedValidationData = relay_chain
					.persisted_validation_data(n.hash(), para_id, OccupiedCoreAssumption::TimedOut)
					.await
					.map(|s| s.map(|s| s))
					.ok()
					.flatten()?;
				Some((n.number, relay_head, n.hash()))
			}
		});

	Ok(new_best_notification_stream)
}

async fn relay_chain_notification<P, R, Block, TBackend>(
	para_id: ParaId,
	parachain: Arc<P>,
	relay_chain: R,
	keystore: KeystorePtr,
	data_path: PathBuf,
	backend: Arc<TBackend>,
) where
	R: RelayChainInterface + Clone,
	Block: BlockT,
	P: ProvideRuntimeApi<Block> + UsageProvider<Block>,
	P::Api: ContainerRuntimeApi<Block, AccountId32>,
	<Block::Header as HeaderT>::Number: Into<u32>,
	TBackend: 'static + sc_client_api::backend::Backend<Block> + Send,
{
	// let mut stop = false;
	// loop{
	// 	if !stop{
	// 		let download_info = DownloadInfo{
	// 			file_name:"magport-node-b".into(),
	// 			app_hash:H256::from_str("9b64d63367328fd980b6e88af0dc46c437bf2c3906a9b000eccd66a6e4599938").
	// unwrap(), 			..Default::default()
	// 		};
	// 		download_sdk(data_path.clone(), download_info, "http://43.134.60.202:88/static").await;
	// 		stop = true;
	// 	}
	// }
	let new_best_heads = match new_best_heads(relay_chain.clone(), para_id).await {
		Ok(best_heads_stream) => best_heads_stream.fuse(),
		Err(_err) => {
			return;
		},
	};
	pin_mut!(new_best_heads);
	let runing_app = Arc::new(Mutex::new(RunningApp {
		group_id: 0xFFFFFFFF,
		app_id: 0xFFFFFFFF,
		app_hash: Default::default(),
		running: RunStatus::Pending,
		app_info: None,
		instance1: None,
		instance2: None,
		instance1_docker: false,
		instance2_docker: false,
		instance1_docker_name: None,
		instance2_docker_name: None,
		cur_ins: InstanceIndex::Instance1,
	}));
	let runing_processor = Arc::new(Mutex::new(RunningProcessor { processors: HashMap::new() }));
	loop {
		select! {
			h = new_best_heads.next() => {
				match h {
					Some((_height, head, _hash)) => {
						let _ = handle_new_best_parachain_head(head, &*parachain,keystore.clone(), data_path.clone(), runing_app.clone(),runing_processor.clone(), backend.clone()).await;
					},
					None => {
						return;
					}
				}
			},
		}
	}
}

pub async fn run_container_task<P, R, Block, TBackend>(
	para_id: ParaId,
	parachain: Arc<P>,
	relay_chain: R,
	keystore: KeystorePtr,
	data_path: PathBuf,
	backend: Arc<TBackend>,
) where
	R: RelayChainInterface + Clone,
	Block: BlockT,
	P: ProvideRuntimeApi<Block> + UsageProvider<Block>,
	P::Api: ContainerRuntimeApi<Block, AccountId32>,
	<Block::Header as HeaderT>::Number: Into<u32>,
	TBackend: 'static + sc_client_api::backend::Backend<Block> + Send,
{
	let relay_chain_notification = relay_chain_notification(
		para_id,
		parachain.clone(),
		relay_chain,
		keystore,
		data_path,
		backend,
	);
	select! {
		_ = relay_chain_notification.fuse() => {},
	}
}

pub fn spawn_container_task<T, R, Block, TBackend>(
	parachain: Arc<T>,
	para_id: ParaId,
	relay_chain: R,
	task_manager: &TaskManager,
	keystore: KeystorePtr,
	data_path: PathBuf,
	backend: Arc<TBackend>,
) -> sc_service::error::Result<()>
where
	Block: BlockT,
	R: RelayChainInterface + Clone + 'static,
	T: Send + Sync + 'static + ProvideRuntimeApi<Block> + UsageProvider<Block>,
	T::Api: ContainerRuntimeApi<Block, AccountId32>,
	<Block::Header as HeaderT>::Number: Into<u32>,
	TBackend: 'static + sc_client_api::backend::Backend<Block> + Send,
{
	let container_task = run_container_task(
		para_id,
		parachain.clone(),
		relay_chain.clone(),
		keystore,
		data_path,
		backend,
	);
	task_manager
		.spawn_essential_handle()
		.spawn_blocking("container_task", None, container_task);
	Ok(())
}
