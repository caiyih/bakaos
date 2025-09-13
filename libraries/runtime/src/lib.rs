#![cfg_attr(
    not(runtime_std),
    no_std,
    allow(internal_features),
    feature(core_intrinsics, linkage)
)]

#[cfg(all(not(runtime_std), target_arch = "riscv64"))]
pub mod baremetal;
mod hosted;

#[macro_use]
#[allow(unused_imports)] // macros from `alloc` are not used on all platforms
#[cfg(not(runtime_std))]
extern crate alloc as alloc_crate;

#[cfg(not(runtime_std))]
mod std_compat;

#[cfg(not(runtime_std))]
pub use std_compat::*;

// Use std directly when available
#[cfg(runtime_std)]
pub use ::std::*;

#[cfg(feature = "boot")]
mod entry;

#[cfg(feature = "boot")]
pub use entry::*;

pub use hermit_sync;
