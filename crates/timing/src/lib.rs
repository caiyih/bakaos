#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

pub const NSEC_PER_SEC: i64 = 1_000_000_000;
pub const USEC_PER_SEC: i64 = 1_000_000;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSpec {
    // second part of the times
    pub tv_sec: i64,
    // nanosecond part of the times
    pub tv_nsec: i64,
}

impl TimeSpec {
    pub fn new(sec: i64, nsec: i64) -> TimeSpec {
        TimeSpec {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }

    pub fn from_ticks(ticks: i64, freq: u64) -> TimeSpec {
        let sec = ticks / freq as i64;
        let nsec = (ticks % freq as i64) * NSEC_PER_SEC / freq as i64;

        TimeSpec {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }

    pub fn add_nanos(&mut self, nanos: i64) {
        self.tv_sec += nanos / NSEC_PER_SEC;
        self.tv_nsec += nanos % NSEC_PER_SEC;

        self.tv_sec += self.tv_nsec / NSEC_PER_SEC;
        self.tv_nsec %= NSEC_PER_SEC;
    }

    pub fn total_seconds(&self) -> f64 {
        self.tv_sec as f64 + self.tv_nsec as f64 / NSEC_PER_SEC as f64
    }

    pub fn total_milliseconds(&self) -> f64 {
        self.tv_sec as f64 * 1_000.0 + self.tv_nsec as f64 / 1_000_000.0
    }

    pub fn to_timeval(&self) -> TimeVal {
        let sec = self.tv_sec + self.tv_nsec / NSEC_PER_SEC;
        let msec = self.tv_nsec % NSEC_PER_SEC / 1_000;

        TimeVal {
            tv_sec: sec,
            tv_msec: msec,
        }
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
                tv_msec: 500_000,
            }
        );
    }
}

// TODO: Add test coverage for TimeVal
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeVal {
    // second part of the times
    pub tv_sec: i64,
    // microsecond part of the times
    pub tv_msec: i64,
}

impl TimeVal {
    pub fn new(sec: i64, msec: i64) -> TimeVal {
        TimeVal {
            tv_sec: sec,
            tv_msec: msec,
        }
    }

    pub fn from_ticks(ticks: i64, freq: u64) -> TimeVal {
        let sec = ticks / freq as i64;
        let msec = (ticks % freq as i64) * USEC_PER_SEC / freq as i64;

        TimeVal {
            tv_sec: sec,
            tv_msec: msec,
        }
    }

    pub fn add_usec(&mut self, usec: i64) {
        self.tv_sec += usec / USEC_PER_SEC;
        self.tv_msec += usec % USEC_PER_SEC;

        self.tv_sec += self.tv_msec / USEC_PER_SEC;
        self.tv_msec %= USEC_PER_SEC;
    }

    pub fn total_seconds(&self) -> f64 {
        self.tv_sec as f64 + self.tv_msec as f64 / USEC_PER_SEC as f64
    }

    pub fn total_milliseconds(&self) -> f64 {
        self.tv_sec as f64 * 1_000.0 + (self.tv_msec as f64 / (USEC_PER_SEC / 1_000) as f64)
    }

    pub fn to_timespec(&self) -> TimeSpec {
        let nsec = self.tv_msec * 1_000;
        let sec = self.tv_sec + nsec / NSEC_PER_SEC;

        TimeSpec {
            tv_sec: sec,
            tv_nsec: nsec % NSEC_PER_SEC,
        }
    }
}

impl core::ops::Add for TimeVal {
    type Output = TimeVal;

    fn add(self, rhs: Self) -> Self::Output {
        let mut time = self;
        time.add_usec(rhs.tv_msec);
        time.tv_sec += rhs.tv_sec;
        time
    }
}

impl core::ops::AddAssign for TimeVal {
    fn add_assign(&mut self, rhs: Self) {
        self.tv_sec += rhs.tv_sec;
        self.add_usec(rhs.tv_msec);
    }
}

impl core::ops::Sub for TimeVal {
    type Output = TimeVal;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut time = self;
        time.tv_sec -= rhs.tv_sec;
        time.add_usec(-rhs.tv_msec);
        time
    }
}

impl core::ops::SubAssign for TimeVal {
    fn sub_assign(&mut self, rhs: Self) {
        self.tv_sec -= rhs.tv_sec;
        self.add_usec(-rhs.tv_msec);
    }
}

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

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSpan {
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

        let total_microseconds = diff_sec * (TICKS_PER_SECOND / TICKS_PER_MICROSECOND)
            + diff_nsec / (NSEC_PER_SEC / TICKS_PER_MICROSECOND);

        TimeSpan {
            _ticks: total_microseconds * TICKS_PER_MICROSECOND,
        }
    }

    pub fn from_timeval_diff(lhs: &TimeVal, rhs: &TimeVal) -> TimeSpan {
        let diff_sec = lhs.tv_sec - rhs.tv_sec;
        let diff_usec = lhs.tv_msec - rhs.tv_msec;

        let total_microseconds = diff_sec * (TICKS_PER_SECOND / TICKS_PER_MICROSECOND)
            + diff_usec / (USEC_PER_SEC / TICKS_PER_MICROSECOND);

        TimeSpan {
            _ticks: total_microseconds * TICKS_PER_MICROSECOND,
        }
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

    // Extract the total days from the TimeSpan
    pub fn nanoseconds(&self) -> i32 {
        (self._ticks % TICKS_PER_MICROSECOND % 100) as i32
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
