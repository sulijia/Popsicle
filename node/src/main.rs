//! Substrate Parachain Node CLI

#![warn(missing_docs)]

mod chain_spec;
mod cli;
mod command;
mod container_task;
mod rpc;
mod service;

fn main() -> sc_cli::Result<()> {
	command::run()
}
