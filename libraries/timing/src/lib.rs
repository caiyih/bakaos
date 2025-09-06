//! # Timing Library
//!
//! A comprehensive timing library providing high-precision time structures and utilities
//! for system programming and time calculations.
//!
//! This library provides three main types for time representation:
//!
//! - [`TimeSpec`]: POSIX-compatible time structure with nanosecond precision
//! - [`TimeVal`]: POSIX-compatible time structure with microsecond precision  
//! - [`TimeSpan`]: Duration-based time structure with 100-nanosecond tick precision
//!
//! ## Features
//!
//! - **High precision**: Support for nanosecond-level time precision
//! - **POSIX compatibility**: TimeSpec and TimeVal are compatible with system structures
//! - **Rich arithmetic**: Full support for time arithmetic operations
//! - **Conversions**: Convert between different time representations
//! - **Standard library integration**: When the `std` feature is enabled, provides
//!   conversions to/from `SystemTime`, `Duration`, and `Instant`
//!
//! ## Examples
//!
//! ```
//! use timing::{TimeSpec, TimeVal, TimeSpan};
//!
//! // Create time values
//! let ts = TimeSpec::new(1, 500_000_000); // 1.5 seconds
//! let tv = TimeVal::new(2, 750_000);      // 2.75 seconds
//! let span = TimeSpan::from_seconds_f64(3.25); // 3.25 seconds
//!
//! // Perform arithmetic
//! let sum = ts + TimeSpec::new(0, 500_000_000);
//! assert_eq!(sum.total_seconds(), 2.0);
//!
//! // Convert between types
//! let tv_from_ts = ts.to_timeval();
//! let ts_from_tv = tv.to_timespec();
//! ```
//!
//! ## Feature Flags
//!
//! - `std`: Enables conversions to/from standard library time types
//! - `no_std`: Default feature for no-std environments

#![cfg_attr(not(feature = "std"), no_std)]

mod timespec;
pub use timespec::TimeSpec;

mod timeval;
pub use timeval::TimeVal;

mod timespan;
pub use timespan::TimeSpan;

/// Number of nanoseconds in one second
pub const NSEC_PER_SEC: i64 = 1_000_000_000;
/// Number of microseconds in one second  
pub const USEC_PER_SEC: i64 = 1_000_000;

// Standard library conversions (only when std feature is enabled)
#[cfg(feature = "std")]
mod std_conversions;
