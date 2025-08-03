#![feature(allocator_api)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(feature = "std")]
pub mod fs;
pub mod kernel;
pub mod allocation;
pub mod memory;
pub mod task;
