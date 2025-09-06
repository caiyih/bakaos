//! # Utilities
//!
//! This crate provides utility types and functions for the BakaOS kernel.
//! 
//! ## Features
//! 
//! - **RAII Cleanup**: The [`InvokeOnDrop`] type provides automatic cleanup functionality
//!   by invoking a closure when the value goes out of scope, ensuring proper resource
//!   management in no_std environments.
//!
//! ## no_std Support
//!
//! This crate is designed to work in `no_std` environments by default, making it suitable
//! for kernel-level code. When running tests, the standard library is available.
//!
//! ## Example
//!
//! ```
//! use utilities::InvokeOnDrop;
//!
//! // Automatically cleanup when guard goes out of scope
//! let _guard = InvokeOnDrop::new(|_| {
//!     // cleanup code here
//!     println!("Cleanup executed!");
//! });
//! // cleanup closure is called when _guard is dropped
//! ```

#![cfg_attr(not(test), no_std)]

mod invoke_on_drop;

pub use invoke_on_drop::*;
