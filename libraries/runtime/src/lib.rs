#![cfg_attr(
    not(feature = "std"),
    no_std,
    allow(internal_features),
    feature(core_intrinsics)
)]

mod baremetal;
mod hosted;

#[cfg(not(feature = "std"))]
#[allow(unused_imports)] // macros from `alloc` are not used on all platforms
#[macro_use]
extern crate alloc as alloc_crate;

#[cfg(not(feature = "std"))]
mod std_compat;

#[cfg(not(feature = "std"))]
pub use std_compat::*;

// Use std directly when available
#[cfg(feature = "std")]
pub use ::std::*;

#[cfg(feature = "boot")]
mod entry;

#[cfg(feature = "boot")]
pub use entry::*;
