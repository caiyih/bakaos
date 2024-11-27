#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod status;
mod structs;
mod tid;
mod uesr_task;

pub use status::TaskStatus;
pub use structs::*;
pub use tid::{allocate_tid, TrackedTaskId};
pub use uesr_task::*;
