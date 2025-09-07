use crate::{TimeVal, NSEC_PER_SEC};

/// A time specification structure representing time as seconds and nanoseconds.
///
/// This structure is compatible with the POSIX `timespec` structure and is
/// commonly used in system programming for high-precision time representation.
///
/// # Examples
///
/// ```
/// use timing::TimeSpec;
///
/// // Create a TimeSpec representing 1.5 seconds
/// let ts = TimeSpec::new(1, 500_000_000);
/// assert_eq!(ts.total_seconds(), 1.5);
///
/// // Create using constructor methods
/// let zero = TimeSpec::zero();
/// assert!(zero.is_zero());
///
/// // Arithmetic operations
/// let ts1 = TimeSpec::new(2, 0);
/// let ts2 = TimeSpec::new(1, 500_000_000);
/// let sum = ts1 + ts2;
/// assert_eq!(sum.total_seconds(), 3.5);
/// ```
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSpec {
    /// Seconds component of the time
    pub tv_sec: i64,
    /// Nanoseconds component of the time (0-999,999,999)
    pub tv_nsec: i64,
}

impl TimeSpec {
    /// Create a new TimeSpec with the given seconds and nanoseconds.
    ///
    /// # Arguments
    /// * `sec` - Seconds component
    /// * `nsec` - Nanoseconds component; normalized into [0, NSEC_PER_SEC) via Euclidean division
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let ts = TimeSpec::new(10, 500_000_000);
    /// assert_eq!(ts.tv_sec, 10);
    /// assert_eq!(ts.tv_nsec, 500_000_000);
    ///
    /// // Test normalization
    /// let ts2 = TimeSpec::new(5, 1_500_000_000); // 1.5 extra seconds
    /// assert_eq!(ts2.tv_sec, 6);
    /// assert_eq!(ts2.tv_nsec, 500_000_000);
    /// ```
    pub fn new(sec: i64, nsec: i64) -> TimeSpec {
        let sec = sec + nsec.div_euclid(NSEC_PER_SEC);
        let nsec = nsec.rem_euclid(NSEC_PER_SEC);
        TimeSpec {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }

    /// Create a new TimeSpec with the given seconds and nanoseconds, without checking for validity
    /// or normalization.
    ///
    /// # Arguments
    /// * `sec` - Seconds component
    /// * `nsec` - Nanoseconds component; normalized into [0, NSEC_PER_SEC) via Euclidean division
    ///
    /// # Safety
    /// This function does not check whether the provided values are valid.
    /// It is the caller's responsibility to ensure that the values are within the allowed range.
    #[inline]
    pub const fn new_unchecked(sec: i64, nsec: i64) -> TimeSpec {
        TimeSpec {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }

    /// Check whether this TimeSpec represents a POSIX time specification.
    ///
    /// # Returns
    /// `true` if the time specification is valid, `false` otherwise.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    ///
    /// let ts = TimeSpec::new_unchecked(1, 2_000_000_000);
    /// assert!(!ts.is_posix());
    ///
    /// let ts = TimeSpec::new(1, 2_000_000_000); // Normalized internally
    /// assert!(ts.is_posix());
    /// ```
    #[inline]
    pub fn is_posix(&self) -> bool {
        self.tv_sec >= 0 && self.tv_nsec >= 0 && self.tv_nsec < NSEC_PER_SEC
    }

    /// Create a TimeSpec representing zero time (0.0 seconds).
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let zero = TimeSpec::zero();
    /// assert!(zero.is_zero());
    /// ```
    #[inline]
    pub fn zero() -> TimeSpec {
        TimeSpec {
            tv_sec: 0,
            tv_nsec: 0,
        }
    }

    /// Create a TimeSpec from clock ticks and frequency.
    ///
    /// # Arguments
    /// * `ticks` - Number of ticks
    /// * `freq` - Frequency in Hz
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// // 1000 ticks at 1000 Hz = 1 second
    /// let ts = TimeSpec::from_ticks(1000, 1000);
    /// assert_eq!(ts.tv_sec, 1);
    /// assert_eq!(ts.tv_nsec, 0);
    /// ```
    pub fn from_ticks(ticks: i64, freq: u64) -> TimeSpec {
        assert!(freq > 0, "freq must be > 0");
        let f = freq as i64;
        let sec = ticks.div_euclid(f);
        let rem = ticks.rem_euclid(f);
        let nsec = ((rem as i128) * (NSEC_PER_SEC as i128) / (f as i128)) as i64;
        TimeSpec {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }

    /// Add nanoseconds to this TimeSpec, handling overflow correctly.
    ///
    /// # Arguments
    /// * `nanos` - Nanoseconds to add (can be negative)
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let mut ts = TimeSpec::new(1, 500_000_000);
    /// ts.add_nanos(700_000_000);
    /// assert_eq!(ts.tv_sec, 2);
    /// assert_eq!(ts.tv_nsec, 200_000_000);
    /// ```
    pub fn add_nanos(&mut self, nanos: i64) {
        let total = self.tv_nsec + nanos;
        self.tv_sec += total.div_euclid(NSEC_PER_SEC);
        self.tv_nsec = total.rem_euclid(NSEC_PER_SEC);
    }

    /// Get the total time as seconds (with fractional part).
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let ts = TimeSpec::new(2, 500_000_000);
    /// assert_eq!(ts.total_seconds(), 2.5);
    /// ```
    #[inline]
    pub fn total_seconds(&self) -> f64 {
        self.tv_sec as f64 + self.tv_nsec as f64 / NSEC_PER_SEC as f64
    }

    /// Get the total time as milliseconds (with fractional part).
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let ts = TimeSpec::new(1, 500_000_000);
    /// assert_eq!(ts.total_milliseconds(), 1500.0);
    /// ```
    #[inline]
    pub fn total_milliseconds(&self) -> f64 {
        self.tv_sec as f64 * 1_000.0 + self.tv_nsec as f64 / 1_000_000.0
    }

    /// Convert this TimeSpec to a TimeVal.
    ///
    /// Note: This conversion may lose precision as TimeVal uses microseconds
    /// while TimeSpec uses nanoseconds.
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let ts = TimeSpec::new(1, 500_000_000);
    /// let tv = ts.to_timeval();
    /// assert_eq!(tv.tv_sec, 1);
    /// assert_eq!(tv.tv_usec, 500_000);
    /// ```
    pub fn to_timeval(&self) -> TimeVal {
        let sec = self.tv_sec + self.tv_nsec / NSEC_PER_SEC;
        let usec = self.tv_nsec % NSEC_PER_SEC / 1_000;

        TimeVal {
            tv_sec: sec,
            tv_usec: usec,
        }
    }

    /// Get total nanoseconds as i64
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let ts = TimeSpec::new(1, 500_000_000);
    /// assert_eq!(ts.total_nanoseconds(), 1_500_000_000);
    /// ```
    #[inline]
    pub fn total_nanoseconds(&self) -> i64 {
        self.tv_sec * NSEC_PER_SEC + self.tv_nsec
    }

    /// Get total microseconds as f64
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let ts = TimeSpec::new(1, 500_000_000);
    /// assert_eq!(ts.total_microseconds(), 1_500_000.0);
    /// ```
    #[inline]
    pub fn total_microseconds(&self) -> f64 {
        self.tv_sec as f64 * 1_000_000.0 + self.tv_nsec as f64 / 1_000.0
    }

    /// Check if this TimeSpec represents zero time
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let zero = TimeSpec::zero();
    /// assert!(zero.is_zero());
    /// let non_zero = TimeSpec::new(1, 0);
    /// assert!(!non_zero.is_zero());
    /// ```
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.tv_sec == 0 && self.tv_nsec == 0
    }

    /// Check if this TimeSpec represents positive time
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let pos = TimeSpec::new(1, 0);
    /// assert!(pos.is_positive());
    /// let zero = TimeSpec::zero();
    /// assert!(!zero.is_positive());
    /// ```
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.tv_sec > 0 || (self.tv_sec == 0 && self.tv_nsec > 0)
    }

    /// Check if this TimeSpec represents negative time
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let neg = TimeSpec::new(-1, 0);
    /// assert!(neg.is_negative());
    /// let pos = TimeSpec::new(1, 0);
    /// assert!(!pos.is_negative());
    /// ```
    #[inline]
    pub fn is_negative(&self) -> bool {
        self.tv_sec < 0 || (self.tv_sec == 0 && self.tv_nsec < 0)
    }

    /// Get the absolute value of this TimeSpec
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let neg = TimeSpec::new(-2, -500_000_000);
    /// let abs_val = neg.abs();
    /// assert_eq!(abs_val.tv_sec, 2);
    /// assert_eq!(abs_val.tv_nsec, 500_000_000);
    /// ```
    pub fn abs(&self) -> TimeSpec {
        let total = (self.tv_sec as i128) * (NSEC_PER_SEC as i128) + (self.tv_nsec as i128);
        let total = total.abs();
        let sec = (total / (NSEC_PER_SEC as i128)) as i64;
        let nsec = (total % (NSEC_PER_SEC as i128)) as i64;
        TimeSpec {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }

    /// Add seconds to this TimeSpec
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let mut ts = TimeSpec::new(1, 0);
    /// ts.add_seconds(2);
    /// assert_eq!(ts.tv_sec, 3);
    /// ```
    #[inline]
    pub fn add_seconds(&mut self, seconds: i64) {
        self.tv_sec += seconds;
    }

    /// Add milliseconds to this TimeSpec
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let mut ts = TimeSpec::new(1, 0);
    /// ts.add_milliseconds(500);
    /// assert_eq!(ts.tv_sec, 1);
    /// assert_eq!(ts.tv_nsec, 500_000_000);
    /// ```
    #[inline]
    pub fn add_milliseconds(&mut self, milliseconds: i64) {
        self.add_nanos(milliseconds * 1_000_000);
    }

    /// Add microseconds to this TimeSpec
    ///
    /// # Examples
    /// ```
    /// use timing::TimeSpec;
    /// let mut ts = TimeSpec::new(1, 0);
    /// ts.add_microseconds(500_000);
    /// assert_eq!(ts.tv_sec, 1);
    /// assert_eq!(ts.tv_nsec, 500_000_000);
    /// ```
    #[inline]
    pub fn add_microseconds(&mut self, microseconds: i64) {
        self.add_nanos(microseconds * 1_000);
    }
}

impl core::ops::Add<TimeSpec> for TimeSpec {
    type Output = TimeSpec;

    fn add(self, other: TimeSpec) -> TimeSpec {
        let mut time = self;
        time.tv_sec += other.tv_sec;
        time.add_nanos(other.tv_nsec);
        time
    }
}

impl core::ops::AddAssign<TimeSpec> for TimeSpec {
    fn add_assign(&mut self, other: TimeSpec) {
        self.tv_sec += other.tv_sec;
        self.add_nanos(other.tv_nsec);
    }
}

impl core::ops::Sub<TimeSpec> for TimeSpec {
    type Output = TimeSpec;

    fn sub(self, other: TimeSpec) -> TimeSpec {
        let mut time = self;
        time.tv_sec -= other.tv_sec;
        time.add_nanos(-other.tv_nsec);
        time
    }
}

impl core::ops::SubAssign<TimeSpec> for TimeSpec {
    fn sub_assign(&mut self, other: TimeSpec) {
        self.tv_sec -= other.tv_sec;
        self.add_nanos(-other.tv_nsec);
    }
}

#[cfg(test)]
mod test_timespec {
    use super::TimeSpec;

    #[test]
    fn test_add_nanos() {
        let mut time = TimeSpec {
            tv_sec: 0,
            tv_nsec: 500_000_000,
        };

        time.add_nanos(500_000_000);

        assert_eq!(
            time,
            TimeSpec {
                tv_sec: 1,
                tv_nsec: 0,
            }
        );
    }

    #[test]
    fn test_add_nanos_with_negative() {
        let mut time = TimeSpec {
            tv_sec: 1,
            tv_nsec: 500_000_000,
        };

        time.add_nanos(-1_000_000_000);

        assert_eq!(
            time,
            TimeSpec {
                tv_sec: 0,
                tv_nsec: 500_000_000,
            }
        );
    }

    #[test]
    fn test_total_seconds() {
        let time = TimeSpec {
            tv_sec: 1,
            tv_nsec: 500_000_000,
        };

        assert_eq!(time.total_seconds(), 1.5);
    }

    #[test]
    fn test_total_milliseconds() {
        let time = TimeSpec {
            tv_sec: 1,
            tv_nsec: 500_000_000,
        };

        assert_eq!(time.total_milliseconds(), 1500.0);
    }

    #[test]
    fn test_add_timespec() {
        let time1 = TimeSpec {
            tv_sec: 1,
            tv_nsec: 500_000_000,
        };
        let time2 = TimeSpec {
            tv_sec: 2,
            tv_nsec: 500_000_000,
        };

        let result = time1 + time2;

        assert_eq!(
            result,
            TimeSpec {
                tv_sec: 4,
                tv_nsec: 0,
            }
        );
    }

    #[test]
    fn test_sub_timespec() {
        let time1 = TimeSpec {
            tv_sec: 3,
            tv_nsec: 500_000_000,
        };
        let time2 = TimeSpec {
            tv_sec: 1,
            tv_nsec: 500_000_000,
        };

        let result = time1 - time2;

        assert_eq!(
            result,
            TimeSpec {
                tv_sec: 2,
                tv_nsec: 0,
            }
        );
    }

    #[test]
    fn test_to_timeval() {
        let time = TimeSpec {
            tv_sec: 1,
            tv_nsec: 1_500_000_000,
        };

        let timeval = time.to_timeval();

        assert_eq!(
            timeval,
            crate::TimeVal {
                tv_sec: 2,
                tv_usec: 500_000,
            }
        );
    }

    #[test]
    fn test_new() {
        let ts = TimeSpec::new(5, 123_456_789);
        assert_eq!(ts.tv_sec, 5);
        assert_eq!(ts.tv_nsec, 123_456_789);
    }

    #[test]
    fn test_zero() {
        let ts = TimeSpec::zero();
        assert_eq!(ts.tv_sec, 0);
        assert_eq!(ts.tv_nsec, 0);
    }

    #[test]
    fn test_from_ticks() {
        let ts = TimeSpec::from_ticks(2_000_000, 1_000_000); // 2 seconds at 1MHz
        assert_eq!(ts.tv_sec, 2);
        assert_eq!(ts.tv_nsec, 0);

        let ts2 = TimeSpec::from_ticks(1_500_000, 1_000_000); // 1.5 seconds at 1MHz
        assert_eq!(ts2.tv_sec, 1);
        assert_eq!(ts2.tv_nsec, 500_000_000);
    }

    #[test]
    fn test_add_nanos_large() {
        let mut time = TimeSpec::new(0, 0);
        time.add_nanos(2_500_000_000); // 2.5 seconds

        assert_eq!(time.tv_sec, 2);
        assert_eq!(time.tv_nsec, 500_000_000);
    }

    #[test]
    fn test_add_nanos_negative_underflow() {
        let mut time = TimeSpec::new(2, 300_000_000);
        time.add_nanos(-2_500_000_000); // -2.5 seconds

        assert_eq!(time.tv_sec, -1);
        assert_eq!(time.tv_nsec, 800_000_000);
    }

    #[test]
    fn test_total_microseconds() {
        let time = TimeSpec::new(1, 500_000_000);
        // 1.5 seconds = 1,500,000 microseconds
        assert_eq!(time.total_microseconds(), 1_500_000.0);
    }

    #[test]
    fn test_total_nanoseconds() {
        let time = TimeSpec::new(1, 500_000_000);
        // 1.5 seconds = 1,500,000,000 nanoseconds
        assert_eq!(time.total_nanoseconds(), 1_500_000_000);
    }

    #[test]
    fn test_abs_normalized_negative() {
        let ts = TimeSpec::new(-1, 200_000_000); // total = -0.8s
        let abs = ts.abs();
        assert_eq!(abs.tv_sec, 0);
        assert_eq!(abs.tv_nsec, 800_000_000);
    }

    #[test]
    fn test_comparison() {
        let ts1 = TimeSpec::new(1, 500_000_000);
        let ts2 = TimeSpec::new(1, 500_000_000);
        let ts3 = TimeSpec::new(2, 0);

        assert_eq!(ts1, ts2);
        assert!(ts1 < ts3);
        assert!(ts3 > ts1);
        assert!(ts1 <= ts2);
        assert!(ts1 >= ts2);
    }

    #[test]
    fn test_add_assign_timespec() {
        let mut ts1 = TimeSpec::new(1, 300_000_000);
        let ts2 = TimeSpec::new(0, 800_000_000);
        ts1 += ts2;
        assert_eq!(ts1.tv_sec, 2);
        assert_eq!(ts1.tv_nsec, 100_000_000);
    }

    #[test]
    fn test_sub_assign_timespec() {
        let mut ts1 = TimeSpec::new(3, 200_000_000);
        let ts2 = TimeSpec::new(1, 800_000_000);
        ts1 -= ts2;
        assert_eq!(ts1.tv_sec, 1);
        assert_eq!(ts1.tv_nsec, 400_000_000);
    }
}
