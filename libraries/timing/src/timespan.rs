/*
    This TimeSpan struct is a derivative work based on .NET Standard Library source code

    All original attributions and licenses apply to this work.

    Adapter: Caiyi Shyu <cai1hsu@outlook.com>

    Source: https://source.dot.net/#System.Private.CoreLib/src/libraries/System.Private.CoreLib/src/System/TimeSpan.cs

    The following descriptions were from the .NET source code, see the original source code for more information.

    TimeSpan represents a duration of time.  A TimeSpan can be negative
    or positive.

    TimeSpan is internally represented as a number of ticks. A tick is equal
    to 100 nanoseconds. While this maps well into units of time such as hours
    and days, any periods longer than that aren't representable in a nice fashion.
    For instance, a month can be between 28 and 31 days, while a year
    can contain 365 or 366 days.  A decade can have between 1 and 3 leapyears,
    depending on when you map the TimeSpan into the calendar.  This is why
    we do not provide Years() or Months().

    Note: System.TimeSpan needs to interop with the WinRT structure
    type Windows::Foundation:TimeSpan. These types are currently binary-compatible in
    memory so no custom marshalling is required. If at any point the implementation
    details of this type should change, or new fields added, we need to remember to add
    an appropriate custom ILMarshaler to keep WInRT interop scenarios enabled.
*/

use crate::{TimeSpec, TimeVal};

// Ticks for TimeSpan per microsecond
// 10
const TICKS_PER_MICROSECOND: i64 = 10;

// Ticks for TimeSpan per millisecond
// 10 * 1000 = 10_000, 1E4
const TICKS_PER_MILLISECOND: i64 = TICKS_PER_MICROSECOND * 1000;

// Ticks for TimeSpan per second
// 10 * 1000 * 1000 = 10_000_000, 1E7
const TICKS_PER_SECOND: i64 = TICKS_PER_MILLISECOND * 1000;

// Ticks for TimeSpan per minute
// 10 * 1000 * 1000 * 60 = 600_000_000, 6E8
const TICKS_PER_MINUTE: i64 = TICKS_PER_SECOND * 60;

// Ticks for TimeSpan per hour
// 10 * 1000 * 1000 * 60 * 60 = 36_000_000_000, 3.6E10
const TICKS_PER_HOUR: i64 = TICKS_PER_MINUTE * 60;

// Ticks for TimeSpan per day
// 10 * 1000 * 1000 * 60 * 60 * 24 = 864_000_000_000, 8.64E11
const TICKS_PER_DAY: i64 = TICKS_PER_HOUR * 24;

// The minimum TimeSpan value.
const MIN_TICKS: i64 = i64::MIN;

const MAX_TICKS: i64 = i64::MAX;

const MIN_MICROSECONDS: i64 = MIN_TICKS / TICKS_PER_MICROSECOND;
const MAX_MICROSECONDS: i64 = MAX_TICKS / TICKS_PER_MICROSECOND;

const MIN_MILLISECONDS: i64 = MIN_TICKS / TICKS_PER_MILLISECOND;
const MAX_MILLISECONDS: i64 = MAX_TICKS / TICKS_PER_MILLISECOND;

// const MIN_SECONDS: i64 = MIN_TICKS / TICKS_PER_SECOND;
// const MAX_SECONDS: i64 = MAX_TICKS / TICKS_PER_SECOND;

// const MIN_MINUTES: i64 = MIN_TICKS / TICKS_PER_MINUTE;
// const MAX_MINUTES: i64 = MAX_TICKS / TICKS_PER_MINUTE;

// const MIN_HOURS: i64 = MIN_TICKS / TICKS_PER_HOUR;
// const MAX_HOURS: i64 = MAX_TICKS / TICKS_PER_HOUR;

// const MIN_DAYS: i64 = MIN_TICKS / TICKS_PER_DAY;
// const MAX_DAYS: i64 = MAX_TICKS / TICKS_PER_DAY;

/// A time span structure representing a duration of time.
///
/// TimeSpan represents a duration of time internally as a number of ticks,
/// where each tick equals 100 nanoseconds. This allows for high-precision
/// time duration calculations and is inspired by .NET's TimeSpan.
///
/// A TimeSpan can be positive or negative, representing forward or backward
/// time durations respectively.
///
/// # Examples
///
/// ```
/// use timing::TimeSpan;
///
/// // Create a TimeSpan representing 1 hour, 30 minutes, 45 seconds
/// let ts = TimeSpan::from_hours_sec(1, 30, 45);
/// assert_eq!(ts.hours(), 1);
/// assert_eq!(ts.minutes(), 30);
/// assert_eq!(ts.seconds(), 45);
///
/// // Create from floating point seconds
/// let ts2 = TimeSpan::from_seconds_f64(1.5);
/// assert_eq!(ts2.total_seconds(), 1.5);
///
/// // Arithmetic operations
/// let sum = ts + ts2;
/// assert!(sum.total_seconds() > 5400.0); // > 1.5 hours
/// ```
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSpan {
    /// Internal representation in ticks (100ns units)
    pub _ticks: i64,
}

// Constructors
impl TimeSpan {
    pub fn zero() -> TimeSpan {
        TimeSpan { _ticks: 0 }
    }

    pub fn max_value() -> TimeSpan {
        TimeSpan { _ticks: MAX_TICKS }
    }

    pub fn min_value() -> TimeSpan {
        TimeSpan { _ticks: MIN_TICKS }
    }

    // The tick is the internal representation of the TimeSpan
    // Not the same as the machine's clock tick
    pub fn from_ticks(ticks: i64) -> TimeSpan {
        TimeSpan { _ticks: ticks }
    }

    pub fn from(
        days: i32,
        hours: i32,
        minutes: i32,
        seconds: i32,
        milliseconds: i32,
        microseconds: i32,
    ) -> TimeSpan {
        let total_microseconds = days as i64 * (TICKS_PER_DAY / TICKS_PER_MICROSECOND)
            + hours as i64 * (TICKS_PER_HOUR / TICKS_PER_MICROSECOND)
            + minutes as i64 * (TICKS_PER_MINUTE / TICKS_PER_MICROSECOND)
            + seconds as i64 * (TICKS_PER_SECOND / TICKS_PER_MICROSECOND)
            + milliseconds as i64 * (TICKS_PER_MILLISECOND / TICKS_PER_MICROSECOND)
            + microseconds as i64;

        // FIXME: This panics the kernel!
        if !(MIN_MICROSECONDS..=MAX_MICROSECONDS).contains(&total_microseconds) {
            panic!("Overflow or underflow");
        }

        TimeSpan {
            _ticks: total_microseconds * TICKS_PER_MICROSECOND,
        }
    }

    pub fn from_days_ms(
        days: i32,
        hours: i32,
        minutes: i32,
        seconds: i32,
        milliseconds: i32,
    ) -> TimeSpan {
        TimeSpan::from(days, hours, minutes, seconds, milliseconds, 0)
    }

    pub fn from_days_sec(days: i32, hours: i32, minutes: i32, seconds: i32) -> TimeSpan {
        TimeSpan::from_days_ms(days, hours, minutes, seconds, 0)
    }

    pub fn from_hours_sec(hours: i32, minutes: i32, seconds: i32) -> TimeSpan {
        TimeSpan::from_days_sec(0, hours, minutes, seconds)
    }
}

impl TimeSpan {
    pub fn from_timespec_diff(lhs: &TimeSpec, rhs: &TimeSpec) -> TimeSpan {
        let diff_sec = lhs.tv_sec - rhs.tv_sec;
        let diff_nsec = lhs.tv_nsec - rhs.tv_nsec;
        // 1 tick = 100ns
        let total_ticks = (diff_sec as i128) * (TICKS_PER_SECOND as i128)
            + (diff_nsec as i128) / 100;
        TimeSpan { _ticks: total_ticks as i64 }
    }

    pub fn from_timeval_diff(lhs: &TimeVal, rhs: &TimeVal) -> TimeSpan {
        let diff_sec = lhs.tv_sec - rhs.tv_sec;
        let diff_usec = lhs.tv_msec - rhs.tv_msec; // treated as microseconds
        let total_ticks = (diff_sec as i128) * (TICKS_PER_SECOND as i128)
            + (diff_usec as i128) * (TICKS_PER_MICROSECOND as i128);
        TimeSpan { _ticks: total_ticks as i64 }
    }
}

// Properties
impl TimeSpan {
    // Extract the ticks from the TimeSpan
    pub fn ticks(&self) -> i64 {
        self._ticks
    }

    // Extract the days from the TimeSpan
    pub fn days(&self) -> i32 {
        (self._ticks / TICKS_PER_DAY) as i32
    }

    // Extract the hours from the TimeSpan
    pub fn hours(&self) -> i32 {
        ((self._ticks / TICKS_PER_HOUR) % 24) as i32
    }

    // Extract the minutes from the TimeSpan
    pub fn minutes(&self) -> i32 {
        ((self._ticks / TICKS_PER_MINUTE) % 60) as i32
    }

    // Extract the seconds from the TimeSpan
    pub fn seconds(&self) -> i32 {
        ((self._ticks / TICKS_PER_SECOND) % 60) as i32
    }

    // Extract the milliseconds from the TimeSpan
    pub fn milliseconds(&self) -> i32 {
        ((self._ticks / TICKS_PER_MILLISECOND) % 1000) as i32
    }

    // Extract the microseconds from the TimeSpan
    pub fn microseconds(&self) -> i32 {
        (self._ticks / TICKS_PER_MICROSECOND % 1000) as i32
    }

    // Extract the nanoseconds within the current microsecond (0..=900, step 100)
    pub fn nanoseconds(&self) -> i32 {
        ((self._ticks.rem_euclid(TICKS_PER_MICROSECOND)) * 100) as i32
    }
}

impl TimeSpan {
    pub fn total_days(&self) -> f64 {
        self._ticks as f64 / TICKS_PER_DAY as f64
    }

    pub fn total_hours(&self) -> f64 {
        self._ticks as f64 / TICKS_PER_HOUR as f64
    }

    pub fn total_minutes(&self) -> f64 {
        self._ticks as f64 / TICKS_PER_MINUTE as f64
    }

    pub fn total_seconds(&self) -> f64 {
        self._ticks as f64 / TICKS_PER_SECOND as f64
    }

    pub fn total_milliseconds(&self) -> f64 {
        let temp: f64 = self._ticks as f64 / TICKS_PER_MILLISECOND as f64;

        if temp > MAX_MILLISECONDS as f64 {
            return MAX_MILLISECONDS as f64;
        }

        if temp < MIN_MILLISECONDS as f64 {
            return MIN_MILLISECONDS as f64;
        }

        temp
    }

    pub fn total_microseconds(&self) -> f64 {
        self._ticks as f64 / TICKS_PER_MICROSECOND as f64
    }

    pub fn total_nanoseconds(&self) -> f64 {
        self._ticks as f64 * 100.0
    }

    /// Check if this TimeSpan is zero
    pub fn is_zero(&self) -> bool {
        self._ticks == 0
    }

    /// Check if this TimeSpan is positive
    pub fn is_positive(&self) -> bool {
        self._ticks > 0
    }

    /// Check if this TimeSpan is negative
    pub fn is_negative(&self) -> bool {
        self._ticks < 0
    }

    /// Get the absolute value of this TimeSpan
    pub fn abs(&self) -> TimeSpan {
        if self.is_negative() {
            TimeSpan {
                _ticks: -self._ticks,
            }
        } else {
            *self
        }
    }

    /// Add another TimeSpan to this one
    pub fn add(&mut self, other: TimeSpan) {
        self._ticks += other._ticks;
    }

    /// Subtract another TimeSpan from this one
    pub fn subtract(&mut self, other: TimeSpan) {
        self._ticks -= other._ticks;
    }

    /// Multiply this TimeSpan by a scalar
    pub fn multiply(&mut self, factor: f64) {
        self._ticks = (self._ticks as f64 * factor) as i64;
    }

    /// Divide this TimeSpan by a scalar
    pub fn divide(&mut self, divisor: f64) {
        if divisor != 0.0 {
            self._ticks = (self._ticks as f64 / divisor) as i64;
        }
    }

    /// Create a TimeSpan from a duration in seconds
    pub fn from_seconds_f64(seconds: f64) -> TimeSpan {
        TimeSpan {
            _ticks: (seconds * TICKS_PER_SECOND as f64) as i64,
        }
    }

    /// Create a TimeSpan from a duration in milliseconds
    pub fn from_milliseconds_f64(milliseconds: f64) -> TimeSpan {
        TimeSpan {
            _ticks: (milliseconds * TICKS_PER_MILLISECOND as f64) as i64,
        }
    }

    /// Create a TimeSpan from a duration in microseconds
    pub fn from_microseconds_f64(microseconds: f64) -> TimeSpan {
        TimeSpan {
            _ticks: (microseconds * TICKS_PER_MICROSECOND as f64) as i64,
        }
    }
}

impl core::ops::Add for TimeSpan {
    type Output = TimeSpan;

    fn add(self, rhs: TimeSpan) -> TimeSpan {
        TimeSpan {
            _ticks: self._ticks + rhs._ticks,
        }
    }
}

impl core::ops::AddAssign for TimeSpan {
    fn add_assign(&mut self, rhs: TimeSpan) {
        self._ticks += rhs._ticks;
    }
}

impl core::ops::Sub for TimeSpan {
    type Output = TimeSpan;

    fn sub(self, rhs: TimeSpan) -> TimeSpan {
        TimeSpan {
            _ticks: self._ticks - rhs._ticks,
        }
    }
}

impl core::ops::SubAssign for TimeSpan {
    fn sub_assign(&mut self, rhs: TimeSpan) {
        self._ticks -= rhs._ticks;
    }
}

#[cfg(test)]
mod test_timespan {
    use super::TimeSpan;
    use crate::{TimeSpec, TimeVal};

    #[test]
    fn test_zero() {
        let ts = TimeSpan::zero();
        assert_eq!(ts._ticks, 0);
    }

    #[test]
    fn test_max_value() {
        let ts = TimeSpan::max_value();
        assert_eq!(ts._ticks, i64::MAX);
    }

    #[test]
    fn test_min_value() {
        let ts = TimeSpan::min_value();
        assert_eq!(ts._ticks, i64::MIN);
    }

    #[test]
    fn test_from_ticks() {
        let ts = TimeSpan::from_ticks(1000);
        assert_eq!(ts._ticks, 1000);
    }

    #[test]
    fn test_from_simple() {
        let ts = TimeSpan::from(1, 2, 3, 4, 5, 6); // 1 day, 2 hours, 3 minutes, 4 seconds, 5 ms, 6 μs
                                                   // 1 day = 864_000_000_000 ticks
                                                   // 2 hours = 72_000_000_000 ticks
                                                   // 3 minutes = 1_800_000_000 ticks
                                                   // 4 seconds = 40_000_000 ticks
                                                   // 5 ms = 50_000 ticks
                                                   // 6 μs = 60 ticks
        let expected = 864_000_000_000 + 72_000_000_000 + 1_800_000_000 + 40_000_000 + 50_000 + 60;
        assert_eq!(ts._ticks, expected);
    }

    #[test]
    fn test_from_days_ms() {
        let ts = TimeSpan::from_days_ms(1, 1, 1, 1, 1);
        let expected_ts = TimeSpan::from(1, 1, 1, 1, 1, 0);
        assert_eq!(ts._ticks, expected_ts._ticks);
    }

    #[test]
    fn test_from_days_sec() {
        let ts = TimeSpan::from_days_sec(1, 1, 1, 1);
        let expected_ts = TimeSpan::from_days_ms(1, 1, 1, 1, 0);
        assert_eq!(ts._ticks, expected_ts._ticks);
    }

    #[test]
    fn test_from_hours_sec() {
        let ts = TimeSpan::from_hours_sec(2, 30, 45);
        let expected_ts = TimeSpan::from_days_sec(0, 2, 30, 45);
        assert_eq!(ts._ticks, expected_ts._ticks);
    }

    #[test]
    fn test_from_timespec_diff() {
        let ts1 = TimeSpec::new(10, 500_000_000);
        let ts2 = TimeSpec::new(5, 200_000_000);
        let diff = TimeSpan::from_timespec_diff(&ts1, &ts2);

        // Expected: 5.3 seconds = 53_000_000 ticks (5.3 * 10_000_000)
        assert_eq!(diff._ticks, 53_000_000);
    }

    #[test]
    fn test_from_timeval_diff() {
        let tv1 = TimeVal::new(10, 500_000);
        let tv2 = TimeVal::new(5, 200_000);
        let diff = TimeSpan::from_timeval_diff(&tv1, &tv2);

        // Expected: 5.3 seconds = 53_000_000 ticks (5.3 * 10_000_000)
        assert_eq!(diff._ticks, 53_000_000);
    }

    #[test]
    fn test_ticks() {
        let ts = TimeSpan::from_ticks(12345);
        assert_eq!(ts.ticks(), 12345);
    }

    #[test]
    fn test_days() {
        let ts = TimeSpan::from(2, 0, 0, 0, 0, 0);
        assert_eq!(ts.days(), 2);
    }

    #[test]
    fn test_hours() {
        let ts = TimeSpan::from(1, 5, 0, 0, 0, 0);
        assert_eq!(ts.hours(), 5);

        // Test hours wrapping (25 hours = 1 day + 1 hour)
        let ts2 = TimeSpan::from(0, 25, 0, 0, 0, 0);
        assert_eq!(ts2.hours(), 1);
        assert_eq!(ts2.days(), 1);
    }

    #[test]
    fn test_minutes() {
        let ts = TimeSpan::from(0, 0, 45, 0, 0, 0);
        assert_eq!(ts.minutes(), 45);

        // Test minutes wrapping
        let ts2 = TimeSpan::from(0, 0, 65, 0, 0, 0);
        assert_eq!(ts2.minutes(), 5);
        assert_eq!(ts2.hours(), 1);
    }

    #[test]
    fn test_seconds() {
        let ts = TimeSpan::from(0, 0, 0, 30, 0, 0);
        assert_eq!(ts.seconds(), 30);

        // Test seconds wrapping
        let ts2 = TimeSpan::from(0, 0, 0, 70, 0, 0);
        assert_eq!(ts2.seconds(), 10);
        assert_eq!(ts2.minutes(), 1);
    }

    #[test]
    fn test_milliseconds() {
        let ts = TimeSpan::from(0, 0, 0, 0, 500, 0);
        assert_eq!(ts.milliseconds(), 500);

        // Test milliseconds wrapping
        let ts2 = TimeSpan::from(0, 0, 0, 0, 1500, 0);
        assert_eq!(ts2.milliseconds(), 500);
        assert_eq!(ts2.seconds(), 1);
    }

    #[test]
    fn test_microseconds() {
        let ts = TimeSpan::from(0, 0, 0, 0, 0, 750);
        assert_eq!(ts.microseconds(), 750);
    }

    #[test]
    fn test_nanoseconds() {
        let ts = TimeSpan::from_ticks(5); // 5 ticks = 500 nanoseconds (5 * 100)
        assert_eq!(ts.nanoseconds(), 500);
    }

    #[test]
    fn test_total_days() {
        let ts = TimeSpan::from(2, 12, 0, 0, 0, 0); // 2.5 days
        assert_eq!(ts.total_days(), 2.5);
    }

    #[test]
    fn test_total_hours() {
        let ts = TimeSpan::from(0, 2, 30, 0, 0, 0); // 2.5 hours
        assert_eq!(ts.total_hours(), 2.5);
    }

    #[test]
    fn test_total_minutes() {
        let ts = TimeSpan::from(0, 0, 2, 30, 0, 0); // 2.5 minutes
        assert_eq!(ts.total_minutes(), 2.5);
    }

    #[test]
    fn test_total_seconds() {
        let ts = TimeSpan::from(0, 0, 0, 2, 500, 0); // 2.5 seconds
        assert_eq!(ts.total_seconds(), 2.5);
    }

    #[test]
    fn test_total_milliseconds() {
        let ts = TimeSpan::from(0, 0, 0, 1, 500, 0); // 1.5 seconds = 1500 ms
        assert_eq!(ts.total_milliseconds(), 1500.0);
    }

    #[test]
    fn test_total_microseconds() {
        let ts = TimeSpan::from(0, 0, 0, 0, 1, 500); // 1500 microseconds
        assert_eq!(ts.total_microseconds(), 1500.0);
    }

    #[test]
    fn test_total_nanoseconds() {
        let ts = TimeSpan::from_ticks(150); // 150 ticks = 15000 nanoseconds
        assert_eq!(ts.total_nanoseconds(), 15000.0);
    }

    #[test]
    fn test_nanoseconds_negative() {
        let ts = TimeSpan::from_ticks(-5);
        assert_eq!(ts.nanoseconds(), 500);
    }

    #[test]
    fn test_from_timespec_diff_sub_micro() {
        let a = TimeSpec::new(0, 0);
        let b = TimeSpec::new(0, 900); // 900ns
        let d = TimeSpan::from_timespec_diff(&a, &b);
        assert_eq!(d.ticks(), -9); // -900ns = -9 ticks
    }

    #[test]
    fn test_add() {
        let ts1 = TimeSpan::from_ticks(1000);
        let ts2 = TimeSpan::from_ticks(500);
        let result = ts1 + ts2;
        assert_eq!(result._ticks, 1500);
    }

    #[test]
    fn test_add_assign() {
        let mut ts1 = TimeSpan::from_ticks(1000);
        let ts2 = TimeSpan::from_ticks(500);
        ts1 += ts2;
        assert_eq!(ts1._ticks, 1500);
    }

    #[test]
    fn test_sub() {
        let ts1 = TimeSpan::from_ticks(1000);
        let ts2 = TimeSpan::from_ticks(300);
        let result = ts1 - ts2;
        assert_eq!(result._ticks, 700);
    }

    #[test]
    fn test_sub_assign() {
        let mut ts1 = TimeSpan::from_ticks(1000);
        let ts2 = TimeSpan::from_ticks(300);
        ts1 -= ts2;
        assert_eq!(ts1._ticks, 700);
    }

    #[test]
    fn test_comparison() {
        let ts1 = TimeSpan::from_ticks(1000);
        let ts2 = TimeSpan::from_ticks(1000);
        let ts3 = TimeSpan::from_ticks(2000);

        assert_eq!(ts1, ts2);
        assert!(ts1 < ts3);
        assert!(ts3 > ts1);
        assert!(ts1 <= ts2);
        assert!(ts1 >= ts2);
    }

    #[test]
    #[should_panic]
    fn test_from_overflow() {
        // This should panic due to overflow during calculation
        TimeSpan::from(i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    }
}
