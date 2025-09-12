//! A `no_std` compatibility layer that re-exports items from `core` and `alloc` as if they were in `std`.
//! This file is manually generated from `rustc/library/std/src/lib.rs`.
//! We are going to add automated generation in the future.

pub use ::core::any;
pub use ::core::array;
pub use ::core::cell;
pub use ::core::char;
pub use ::core::clone;
pub use ::core::cmp;
pub use ::core::convert;
pub use ::core::default;
pub use ::core::future;
pub use ::core::hint;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::i32;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::i64;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::i8;
pub use ::core::intrinsics;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::isize;
pub use ::core::iter;
pub use ::core::marker;
pub use ::core::mem;
pub use ::core::ops;
pub use ::core::option;
pub use ::core::pin;
pub use ::core::ptr;
pub use ::core::*;

pub use ::core::result;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::u128;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::u16;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::u32;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::u64;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::u8;
#[allow(deprecated, deprecated_in_future)]
pub use ::core::usize;

pub use ::alloc_crate::borrow;
pub use ::alloc_crate::boxed;
pub use ::alloc_crate::fmt;
pub use ::alloc_crate::format;
pub use ::alloc_crate::rc;
pub use ::alloc_crate::slice;
pub use ::alloc_crate::str;
pub use ::alloc_crate::string;
pub use ::alloc_crate::sync;
pub use ::alloc_crate::vec;
