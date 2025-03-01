#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

mod timespec;
pub use timespec::TimeSpec;

mod timeval;
pub use timeval::TimeVal;

mod timespan;
pub use timespan::TimeSpan;

pub const NSEC_PER_SEC: i64 = 1_000_000_000;
pub const USEC_PER_SEC: i64 = 1_000_000;
