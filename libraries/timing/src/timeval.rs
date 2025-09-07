//! TimeVal - POSIX-compatible time instant representation with microsecond precision
//!
//! This module provides the `TimeVal` structure for representing time instants
//! with microsecond precision, compatible with the POSIX `timeval` structure.
//!
//! # Time Instant vs Duration Semantics
//!
//! `TimeVal` represents a **time instant** (point in time), not a duration.
//! For duration operations, consider using `TimeSpan` instead. However, arithmetic
//! operations between `TimeVal` instances are provided for convenience:
//!
//! - `TimeVal + TimeVal` = `TimeVal` (semantically questionable, but allowed)
//! - `TimeVal - TimeVal` = `TimeVal` (can represent duration-like difference)
//! - Use `to_timespan()` method to convert to proper duration representation
//!
//! # POSIX Compatibility
//!
//! The `TimeVal` structure is binary-compatible with the POSIX `timeval` structure:
//! ```c
//! struct timeval {
//!     time_t      tv_sec;   // seconds
//!     suseconds_t tv_usec;  // microseconds
//! };
//! ```
//!
//! # Examples
//!
//! ```
//! use timing::{TimeVal, TimeSpan};
//!
//! // Creating time instants
//! let instant1 = TimeVal::new(10, 500_000);    // 10.5 seconds
//! let instant2 = TimeVal::new(5, 250_000);     // 5.25 seconds
//!
//! // Converting to duration for semantic clarity
//! let duration = instant1.to_timespan();
//!
//! // Computing differences (results in time difference)
//! let diff = instant1 - instant2;  // 5.25 seconds difference
//!
//! // Converting between TimeVal and TimeSpec
//! let timespec = instant1.to_timespec();
//! ```

use crate::{TimeSpan, TimeSpec, NSEC_PER_SEC, USEC_PER_SEC};
/// A time value structure representing time as seconds and microseconds.
///
/// This structure is compatible with the POSIX `timeval` structure and is
/// commonly used in system programming for time representation with microsecond precision.
/// It represents a **time instant** (point in time) rather than a duration.
///
/// # POSIX Compatibility
///
/// The layout is binary-compatible with the POSIX `timeval` structure:
/// ```c
/// struct timeval {
///     time_t      tv_sec;   // seconds since Unix epoch
///     suseconds_t tv_usec;  // microseconds (0-999,999)
/// };
/// ```
///
/// # Field Constraints
///
/// - `tv_sec`: Can be any valid i64 value representing seconds
/// - `tv_usec`: Should be in range [0, 999,999] for POSIX compliance,
///   but negative values are handled for intermediate calculations
///
/// # Precision vs TimeSpec
///
/// TimeVal provides microsecond precision (1µs = 0.000001s) while TimeSpec
/// provides nanosecond precision (1ns = 0.000000001s). Use TimeSpec when
/// higher precision is needed.
///
/// # Examples
///
/// ```
/// use timing::{TimeVal, TimeSpan};
///
/// // Create a TimeVal representing 1.5 seconds
/// let tv = TimeVal::new(1, 500_000);
/// assert_eq!(tv.total_seconds(), 1.5);
///
/// // Create using constructor methods
/// let zero = TimeVal::zero();
/// assert!(zero.is_zero());
///
/// // Arithmetic operations (use with caution for semantics)
/// let tv1 = TimeVal::new(2, 0);
/// let tv2 = TimeVal::new(1, 500_000);
/// let sum = tv1 + tv2;
/// assert_eq!(sum.total_seconds(), 3.5);
///
/// // Convert to duration for better semantics
/// let duration = tv.to_timespan();
/// assert_eq!(duration.total_seconds(), 1.5);
///
/// // Convert to TimeSpec for higher precision
/// let ts = tv.to_timespec();
/// assert_eq!(ts.tv_sec, 1);
/// assert_eq!(ts.tv_nsec, 500_000_000);
/// ```
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeVal {
    /// Seconds component of the time
    pub tv_sec: i64,
    /// Microseconds component of the time (0-999,999)
    pub tv_usec: i64,
}

impl TimeVal {
    /// Create a new TimeVal with the given seconds and microseconds.
    ///
    /// # Arguments
    /// * `sec` - Seconds component
    /// * `usec` - Microseconds component (typically 0-999,999 for normalized values)
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let tv = TimeVal::new(10, 500_000);
    /// assert_eq!(tv.tv_sec, 10);
    /// assert_eq!(tv.tv_usec, 500_000);
    /// assert_eq!(tv.total_seconds(), 10.5);
    /// ```
    #[inline]
    pub fn new(sec: i64, usec: i64) -> TimeVal {
        TimeVal {
            tv_sec: sec,
            tv_usec: usec,
        }
    }

    /// Create a TimeVal representing zero time.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let zero = TimeVal::zero();
    /// assert_eq!(zero.tv_sec, 0);
    /// assert_eq!(zero.tv_usec, 0);
    /// assert!(zero.is_zero());
    /// ```
    #[inline]
    pub fn zero() -> TimeVal {
        TimeVal {
            tv_usec: 0,
            tv_sec: 0,
        }
    }

    /// Create a TimeVal from tick count and frequency.
    ///
    /// # Arguments
    /// * `ticks` - Number of ticks
    /// * `freq` - Frequency in Hz (ticks per second)
    ///
    /// # Panics
    /// Panics if `freq` is 0.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let tv = TimeVal::from_ticks(2_000_000, 1_000_000); // 2 seconds at 1MHz
    /// assert_eq!(tv.tv_sec, 2);
    /// assert_eq!(tv.tv_usec, 0);
    /// ```
    pub fn from_ticks(ticks: i64, freq: u64) -> TimeVal {
        assert!(freq > 0, "Frequency cannot be zero");
        let sec = ticks / freq as i64;
        let usec = (ticks % freq as i64) * USEC_PER_SEC / freq as i64;

        TimeVal {
            tv_sec: sec,
            tv_usec: usec,
        }
    }

    /// Add microseconds to this TimeVal with proper overflow handling.
    ///
    /// This method properly handles overflow and underflow of the microseconds
    /// component, adjusting the seconds accordingly.
    ///
    /// # Arguments
    /// * `usec` - Microseconds to add (can be negative)
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let mut tv = TimeVal::new(1, 500_000);
    /// tv.add_usec(500_000);
    /// assert_eq!(tv.tv_sec, 2);
    /// assert_eq!(tv.tv_usec, 0);
    ///
    /// // Test negative addition
    /// let mut tv2 = TimeVal::new(2, 300_000);
    /// tv2.add_usec(-500_000);
    /// assert_eq!(tv2.tv_sec, 1);
    /// assert_eq!(tv2.tv_usec, 800_000);
    /// ```
    pub fn add_usec(&mut self, usec: i64) {
        self.tv_sec += usec / USEC_PER_SEC;
        self.tv_usec += usec % USEC_PER_SEC;

        // Handle overflow/underflow for microseconds
        if self.tv_usec >= USEC_PER_SEC {
            self.tv_sec += self.tv_usec / USEC_PER_SEC;
            self.tv_usec %= USEC_PER_SEC;
        } else if self.tv_usec < 0 {
            let borrow = (-self.tv_usec + USEC_PER_SEC - 1) / USEC_PER_SEC;
            self.tv_sec -= borrow;
            self.tv_usec += borrow * USEC_PER_SEC;
        }
    }

    /// Get total time as seconds in floating point.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let tv = TimeVal::new(2, 500_000);
    /// assert_eq!(tv.total_seconds(), 2.5);
    ///
    /// let tv2 = TimeVal::new(0, 250_000);
    /// assert_eq!(tv2.total_seconds(), 0.25);
    /// ```
    #[inline]
    pub fn total_seconds(&self) -> f64 {
        self.tv_sec as f64 + self.tv_usec as f64 / USEC_PER_SEC as f64
    }

    /// Get total time as milliseconds in floating point.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let tv = TimeVal::new(1, 500_000);
    /// assert_eq!(tv.total_milliseconds(), 1500.0);
    ///
    /// let tv2 = TimeVal::new(0, 250_000);
    /// assert_eq!(tv2.total_milliseconds(), 250.0);
    /// ```
    #[inline]
    pub fn total_milliseconds(&self) -> f64 {
        self.tv_sec as f64 * 1_000.0 + (self.tv_usec as f64 / (USEC_PER_SEC / 1_000) as f64)
    }

    /// Convert this TimeVal to a TimeSpec.
    ///
    /// This conversion increases precision from microseconds to nanoseconds.
    /// The resulting TimeSpec will have properly normalized fields.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let tv = TimeVal::new(1, 500_000);
    /// let ts = tv.to_timespec();
    /// assert_eq!(ts.tv_sec, 1);
    /// assert_eq!(ts.tv_nsec, 500_000_000);
    ///
    /// // Test with overflow
    /// let tv2 = TimeVal::new(0, 1_500_000);
    /// let ts2 = tv2.to_timespec();
    /// assert_eq!(ts2.tv_sec, 1);
    /// assert_eq!(ts2.tv_nsec, 500_000_000);
    /// ```
    pub fn to_timespec(&self) -> TimeSpec {
        let total_ns: i128 =
            (self.tv_sec as i128) * (NSEC_PER_SEC as i128) + (self.tv_usec as i128) * 1_000i128;
        let sec = (total_ns.div_euclid(NSEC_PER_SEC as i128)) as i64;
        let nsec = (total_ns.rem_euclid(NSEC_PER_SEC as i128)) as i64;
        TimeSpec {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }

    /// Convert this TimeVal to a TimeSpan for duration-based operations.
    ///
    /// This method treats the TimeVal as a duration relative to zero time,
    /// which is semantically appropriate for operations that require duration
    /// representation while working with time instant structures.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let tv = TimeVal::new(1, 500_000);  // 1.5 seconds
    /// let span = tv.to_timespan();
    /// assert_eq!(span.total_seconds(), 1.5);
    /// ```
    pub fn to_timespan(&self) -> TimeSpan {
        // Convert to total nanoseconds, then to TimeSpan ticks (100ns units)
        let total_nanos = self.tv_sec * NSEC_PER_SEC + self.tv_usec * 1_000;
        let ticks = total_nanos / 100; // 100 nanoseconds per tick
        TimeSpan::from_ticks(ticks)
    }

    /// Get total microseconds as i64.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let tv = TimeVal::new(1, 500_000);
    /// assert_eq!(tv.total_microseconds(), 1_500_000);
    /// ```
    #[inline]
    pub fn total_microseconds(&self) -> i64 {
        self.tv_sec * USEC_PER_SEC + self.tv_usec
    }

    /// Get total nanoseconds as i64.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let tv = TimeVal::new(1, 500_000);
    /// assert_eq!(tv.total_nanoseconds(), 1_500_000_000);
    /// ```
    #[inline]
    pub fn total_nanoseconds(&self) -> i64 {
        self.tv_sec * NSEC_PER_SEC + self.tv_usec * 1_000
    }

    /// Check if this TimeVal represents zero time.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let zero = TimeVal::zero();
    /// assert!(zero.is_zero());
    /// let non_zero = TimeVal::new(1, 0);
    /// assert!(!non_zero.is_zero());
    /// ```
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.tv_sec == 0 && self.tv_usec == 0
    }

    /// Check if this TimeVal represents a positive time value.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let pos = TimeVal::new(1, 500_000);
    /// assert!(pos.is_positive());
    /// let zero = TimeVal::zero();
    /// assert!(!zero.is_positive());
    /// ```
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.tv_sec > 0 || (self.tv_sec == 0 && self.tv_usec > 0)
    }

    /// Check if this TimeVal represents a negative time value.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let neg = TimeVal::new(-1, -500_000);
    /// assert!(neg.is_negative());
    /// let pos = TimeVal::new(1, 0);
    /// assert!(!pos.is_negative());
    /// ```
    #[inline]
    pub fn is_negative(&self) -> bool {
        self.tv_sec < 0 || (self.tv_sec == 0 && self.tv_usec < 0)
    }

    /// Get the absolute value of this TimeVal.
    ///
    /// Returns a TimeVal with the absolute time value, handling negative
    /// components correctly using total time computation.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let neg = TimeVal::new(-1, -500_000);
    /// let abs_val = neg.abs();
    /// assert_eq!(abs_val.tv_sec, 1);
    /// assert_eq!(abs_val.tv_usec, 500_000);
    /// ```
    pub fn abs(&self) -> TimeVal {
        let total: i128 = (self.tv_sec as i128) * (USEC_PER_SEC as i128) + (self.tv_usec as i128);
        let abs_total = if total < 0 { -total } else { total };
        TimeVal {
            tv_sec: (abs_total / (USEC_PER_SEC as i128)) as i64,
            tv_usec: (abs_total % (USEC_PER_SEC as i128)) as i64,
        }
    }

    /// Add seconds to this TimeVal.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let mut tv = TimeVal::new(1, 500_000);
    /// tv.add_seconds(2);
    /// assert_eq!(tv.tv_sec, 3);
    /// assert_eq!(tv.tv_usec, 500_000);
    /// ```
    #[inline]
    pub fn add_seconds(&mut self, seconds: i64) {
        self.tv_sec += seconds;
    }

    /// Add milliseconds to this TimeVal.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let mut tv = TimeVal::new(1, 0);
    /// tv.add_milliseconds(500);
    /// assert_eq!(tv.tv_sec, 1);
    /// assert_eq!(tv.tv_usec, 500_000);
    /// ```
    #[inline]
    pub fn add_milliseconds(&mut self, milliseconds: i64) {
        self.add_usec(milliseconds * 1_000);
    }
}

impl Default for TimeVal {
    /// Create a default TimeVal (zero time).
    ///
    /// # Examples
    /// ```
    /// use timing::TimeVal;
    /// let tv = TimeVal::default();
    /// assert_eq!(tv, TimeVal::zero());
    /// ```
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

/// Addition operator for TimeVal.
///
/// **Semantic Note**: Adding two time instants is semantically questionable.
/// Consider whether you actually want to add a duration to an instant instead.
/// For duration-based arithmetic, use `TimeSpan` or convert via `to_timespan()`.
///
/// # Examples
/// ```
/// use timing::TimeVal;
/// let tv1 = TimeVal::new(1, 500_000);
/// let tv2 = TimeVal::new(2, 250_000);
/// let sum = tv1 + tv2;  // 3.75 seconds total
/// ```
impl core::ops::Add for TimeVal {
    type Output = TimeVal;

    fn add(self, rhs: Self) -> Self::Output {
        let mut time = self;
        time.add_usec(rhs.tv_usec);
        time.tv_sec += rhs.tv_sec;
        time
    }
}

/// Addition assignment operator for TimeVal.
///
/// **Semantic Note**: Adding time instants is semantically questionable.
/// Consider using duration-based operations instead.
impl core::ops::AddAssign for TimeVal {
    fn add_assign(&mut self, rhs: Self) {
        self.tv_sec += rhs.tv_sec;
        self.add_usec(rhs.tv_usec);
    }
}

/// Subtraction operator for TimeVal.
///
/// **Semantic Note**: This operation computes the difference between two time instants,
/// which can represent a duration. The result is technically a TimeVal but represents
/// a time difference. For proper duration semantics, consider using:
/// ```
/// # use timing::{TimeVal, TimeSpan};
/// let instant1 = TimeVal::new(3, 500_000);
/// let instant2 = TimeVal::new(1, 250_000);
/// let duration = (instant1 - instant2).to_timespan();
/// ```
///
/// # Examples
/// ```
/// use timing::TimeVal;
/// let tv1 = TimeVal::new(3, 500_000);
/// let tv2 = TimeVal::new(1, 250_000);
/// let diff = tv1 - tv2;  // 2.25 seconds difference
/// ```
impl core::ops::Sub for TimeVal {
    type Output = TimeVal;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut time = self;
        time.tv_sec -= rhs.tv_sec;
        time.add_usec(-rhs.tv_usec);
        time
    }
}

/// Subtraction assignment operator for TimeVal.
///
/// **Semantic Note**: Subtracting time instants represents computing a time difference.
/// Consider using duration-based operations for clearer semantics.
impl core::ops::SubAssign for TimeVal {
    fn sub_assign(&mut self, rhs: Self) {
        self.tv_sec -= rhs.tv_sec;
        self.add_usec(-rhs.tv_usec);
    }
}

#[cfg(test)]
mod test_timeval {
    use super::TimeVal;

    #[test]
    fn test_new() {
        let tv = TimeVal::new(10, 500_000);
        assert_eq!(tv.tv_sec, 10);
        assert_eq!(tv.tv_usec, 500_000);
    }

    #[test]
    fn test_zero() {
        let tv = TimeVal::zero();
        assert_eq!(tv.tv_sec, 0);
        assert_eq!(tv.tv_usec, 0);
    }

    #[test]
    fn test_default() {
        let tv = TimeVal::default();
        assert_eq!(tv, TimeVal::zero());
    }

    #[test]
    fn test_from_ticks() {
        let tv = TimeVal::from_ticks(2_000_000, 1_000_000); // 2 seconds at 1MHz
        assert_eq!(tv.tv_sec, 2);
        assert_eq!(tv.tv_usec, 0);

        let tv2 = TimeVal::from_ticks(1_500_000, 1_000_000); // 1.5 seconds at 1MHz
        assert_eq!(tv2.tv_sec, 1);
        assert_eq!(tv2.tv_usec, 500_000);
    }

    #[test]
    fn test_add_usec() {
        let mut tv = TimeVal::new(1, 500_000);
        tv.add_usec(500_000);
        assert_eq!(tv.tv_sec, 2);
        assert_eq!(tv.tv_usec, 0);

        // Test negative addition
        let mut tv2 = TimeVal::new(2, 300_000);
        tv2.add_usec(-500_000);
        assert_eq!(tv2.tv_sec, 1);
        assert_eq!(tv2.tv_usec, 800_000);
    }

    #[test]
    fn test_add_usec_overflow() {
        let mut tv = TimeVal::new(0, 800_000);
        tv.add_usec(300_000);
        assert_eq!(tv.tv_sec, 1);
        assert_eq!(tv.tv_usec, 100_000);
    }

    #[test]
    fn test_total_seconds() {
        let tv = TimeVal::new(2, 500_000);
        assert_eq!(tv.total_seconds(), 2.5);

        let tv2 = TimeVal::new(0, 250_000);
        assert_eq!(tv2.total_seconds(), 0.25);
    }

    #[test]
    fn test_total_milliseconds() {
        let tv = TimeVal::new(1, 500_000);
        assert_eq!(tv.total_milliseconds(), 1500.0);

        let tv2 = TimeVal::new(0, 250_000);
        assert_eq!(tv2.total_milliseconds(), 250.0);
    }

    #[test]
    fn test_to_timespec() {
        let tv = TimeVal::new(1, 500_000);
        let ts = tv.to_timespec();
        assert_eq!(ts.tv_sec, 1);
        assert_eq!(ts.tv_nsec, 500_000_000);

        // Test with overflow
        let tv2 = TimeVal::new(0, 1_500_000);
        let ts2 = tv2.to_timespec();
        assert_eq!(ts2.tv_sec, 1);
        assert_eq!(ts2.tv_nsec, 500_000_000);
    }

    #[test]
    fn test_add_timeval() {
        let tv1 = TimeVal::new(1, 300_000);
        let tv2 = TimeVal::new(2, 800_000);
        let result = tv1 + tv2;
        assert_eq!(result.tv_sec, 4);
        assert_eq!(result.tv_usec, 100_000);
    }

    #[test]
    fn test_add_assign_timeval() {
        let mut tv1 = TimeVal::new(1, 300_000);
        let tv2 = TimeVal::new(0, 800_000);
        tv1 += tv2;
        assert_eq!(tv1.tv_sec, 2);
        assert_eq!(tv1.tv_usec, 100_000);
    }

    #[test]
    fn test_sub_timeval() {
        let tv1 = TimeVal::new(3, 200_000);
        let tv2 = TimeVal::new(1, 800_000);
        let result = tv1 - tv2;
        assert_eq!(result.tv_sec, 1);
        assert_eq!(result.tv_usec, 400_000);
    }

    #[test]
    fn test_sub_assign_timeval() {
        let mut tv1 = TimeVal::new(3, 200_000);
        let tv2 = TimeVal::new(1, 800_000);
        tv1 -= tv2;
        assert_eq!(tv1.tv_sec, 1);
        assert_eq!(tv1.tv_usec, 400_000);
    }

    #[test]
    fn test_comparison() {
        let tv1 = TimeVal::new(1, 500_000);
        let tv2 = TimeVal::new(1, 500_000);
        let tv3 = TimeVal::new(2, 0);

        assert_eq!(tv1, tv2);
        assert!(tv1 < tv3);
        assert!(tv3 > tv1);
        assert!(tv1 <= tv2);
        assert!(tv1 >= tv2);
    }

    #[test]
    fn test_to_timespec_negative_microseconds() {
        let tv = TimeVal::new(-1, -500_000); // -1.5 seconds
        let ts = tv.to_timespec();
        assert!(ts.tv_nsec >= 0 && ts.tv_nsec < 1_000_000_000); // Normalized
    }

    #[test]
    fn test_abs_mixed_sign() {
        let tv = TimeVal::new(-1, 200_000); // net: -0.8 seconds
        let abs_tv = tv.abs();
        assert_eq!(abs_tv.tv_sec, 0);
        assert_eq!(abs_tv.tv_usec, 800_000);
    }
}
