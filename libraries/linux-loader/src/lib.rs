#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod auxv;
mod loader;

pub use loader::*;
