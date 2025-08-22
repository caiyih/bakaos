#![feature(cfg_accessible)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod id_allocator;
mod process;
mod task;

pub use process::*;
pub use task::*;
