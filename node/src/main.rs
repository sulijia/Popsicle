//! Substrate Parachain Node CLI

#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;
mod container_task;
mod rpc;

fn main() -> sc_cli::Result<()> {
	command::run()
}
