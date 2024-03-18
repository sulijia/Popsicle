// Cumulus types re-export
// These types are shared between the popsicle and other runtimes
//https://github.com/paritytech/cumulus/tree/master/parachains/common
pub use parachains_common::Balance;

pub const POPS: Balance = 1_000_000_000_000;
pub const MILLIPOPS: Balance = POPS / 1_000;
pub const MICROPOPS: Balance = MILLIPOPS / 1_000;

pub const EXISTENTIAL_DEPOSIT: Balance = MILLIPOPS;

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * 20 * POPS + (bytes as Balance) * 100 * MICROPOPS) / 100
}
