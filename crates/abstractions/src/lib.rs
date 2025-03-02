#![feature(const_trait_impl)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

pub mod operations;

pub use operations::*;
