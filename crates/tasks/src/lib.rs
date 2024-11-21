#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

mod status;
mod tid;

pub use status::TaskStatus;
pub use tid::{allocate_tid, TrackedTaskId};
