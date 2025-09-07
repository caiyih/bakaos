use crate::{TimeSpan, TimeSpec, TimeVal, NSEC_PER_SEC};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// SystemTime conversions
impl From<SystemTime> for TimeSpec {
    #[inline]
    fn from(system_time: SystemTime) -> Self {
        match system_time.duration_since(UNIX_EPOCH) {
            Ok(duration) => TimeSpec {
                tv_sec: duration.as_secs() as i64,
                tv_nsec: duration.subsec_nanos() as i64,
            },
            Err(e) => {
                // Normalize time before UNIX_EPOCH so tv_nsec ∈ [0, 1e9)
                let dur = e.duration();
                if dur.subsec_nanos() == 0 {
                    TimeSpec {
                        tv_sec: -(dur.as_secs() as i64),
                        tv_nsec: 0,
                    }
                } else {
                    TimeSpec {
                        tv_sec: -(dur.as_secs() as i64) - 1,
                        tv_nsec: NSEC_PER_SEC - dur.subsec_nanos() as i64,
                    }
                }
            }
        }
    }
}

impl From<TimeSpec> for SystemTime {
    #[inline]
    fn from(timespec: TimeSpec) -> Self {
        let total: i128 =
            (timespec.tv_sec as i128) * (NSEC_PER_SEC as i128) + (timespec.tv_nsec as i128);
        if total >= 0 {
            UNIX_EPOCH + Duration::from_nanos(total as u64)
        } else {
            UNIX_EPOCH - Duration::from_nanos((-total) as u64)
        }
    }
}

impl From<SystemTime> for TimeVal {
    #[inline]
    fn from(system_time: SystemTime) -> Self {
        let timespec = TimeSpec::from(system_time);
        timespec.to_timeval()
    }
}

impl From<TimeVal> for SystemTime {
    #[inline]
    fn from(timeval: TimeVal) -> Self {
        let timespec = timeval.to_timespec();
        SystemTime::from(timespec)
    }
}

// Duration conversions
impl From<Duration> for TimeSpec {
    #[inline]
    fn from(duration: Duration) -> Self {
        TimeSpec {
            tv_sec: duration.as_secs() as i64,
            tv_nsec: duration.subsec_nanos() as i64,
        }
    }
}

impl TryFrom<TimeSpec> for Duration {
    type Error = &'static str;

    #[inline]
    fn try_from(timespec: TimeSpec) -> Result<Self, Self::Error> {
        let total: i128 =
            (timespec.tv_sec as i128) * (NSEC_PER_SEC as i128) + (timespec.tv_nsec as i128);
        if total < 0 {
            return Err("Cannot convert negative TimeSpec to Duration");
        }
        Ok(Duration::from_nanos(total as u64))
    }
}

impl From<Duration> for TimeVal {
    #[inline]
    fn from(duration: Duration) -> Self {
        let timespec = TimeSpec::from(duration);
        timespec.to_timeval()
    }
}

impl TryFrom<TimeVal> for Duration {
    type Error = &'static str;

    #[inline]
    fn try_from(timeval: TimeVal) -> Result<Self, Self::Error> {
        let total: i128 = (timeval.tv_sec as i128) * 1_000_000i128 + (timeval.tv_usec as i128);
        if total < 0 {
            return Err("Cannot convert negative TimeVal to Duration");
        }
        Ok(Duration::from_micros(total as u64))
    }
}

impl From<Duration> for TimeSpan {
    #[inline]
    fn from(duration: Duration) -> Self {
        // Convert duration to ticks (100ns units)
        let total_nanos = duration.as_secs() * NSEC_PER_SEC as u64 + duration.subsec_nanos() as u64;
        TimeSpan::from_ticks((total_nanos / 100) as i64)
    }
}

impl TryFrom<TimeSpan> for Duration {
    type Error = &'static str;

    #[inline]
    fn try_from(timespan: TimeSpan) -> Result<Self, Self::Error> {
        if timespan.is_negative() {
            Err("Cannot convert negative TimeSpan to Duration")
        } else {
            // Convert ticks (100ns units) back to Duration
            let total_nanos: i128 = (timespan._ticks as i128) * 100i128;
            if total_nanos > u64::MAX as i128 {
                return Err("TimeSpan too large to convert to Duration");
            }
            Ok(Duration::from_nanos(total_nanos as u64))
        }
    }
}

// Instant conversions (note: Instant cannot be converted TO our types since it's relative)
// But we can create TimeSpan from the difference between two Instants
impl TimeSpan {
    /// Create a TimeSpan from the duration between two Instants
    pub fn from_instant_diff(later: Instant, earlier: Instant) -> TimeSpan {
        if later >= earlier {
            <TimeSpan as From<Duration>>::from(later.duration_since(earlier))
        } else {
            -<TimeSpan as From<Duration>>::from(earlier.duration_since(later))
        }
    }
}

// Add negation for TimeSpan
impl std::ops::Neg for TimeSpan {
    type Output = TimeSpan;

    #[inline]
    fn neg(self) -> Self::Output {
        TimeSpan::from_ticks(-self.ticks())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn test_systemtime_to_timespec() {
        let now = UNIX_EPOCH + Duration::from_secs(1234567890) + Duration::from_nanos(123456789);
        let ts = TimeSpec::from(now);
        assert_eq!(ts.tv_sec, 1234567890);
        assert_eq!(ts.tv_nsec, 123456789);
    }

    #[test]
    fn test_timespec_to_systemtime() {
        let ts = TimeSpec::new(1234567890, 123456789);
        let system_time = SystemTime::from(ts);
        let expected =
            UNIX_EPOCH + Duration::from_secs(1234567890) + Duration::from_nanos(123456789);
        assert_eq!(system_time, expected);
    }

    #[test]
    fn test_duration_to_timespec() {
        let duration = Duration::from_secs(123) + Duration::from_nanos(456789000);
        let ts = TimeSpec::from(duration);
        assert_eq!(ts.tv_sec, 123);
        assert_eq!(ts.tv_nsec, 456789000);
    }

    #[test]
    fn test_timespec_to_duration() {
        let ts = TimeSpec::new(123, 456789000);
        let duration = Duration::try_from(ts).unwrap();
        assert_eq!(duration.as_secs(), 123);
        assert_eq!(duration.subsec_nanos(), 456789000);
    }

    #[test]
    fn test_negative_timespec_to_duration_fails() {
        let ts = TimeSpec::new(-123, 0);
        assert!(Duration::try_from(ts).is_err());
    }

    #[test]
    fn test_duration_to_timespan() {
        let duration = Duration::from_secs(1) + Duration::from_millis(500);
        let ts: TimeSpan = duration.into();
        assert_eq!(ts.total_seconds(), 1.5);
    }

    #[test]
    fn test_timespan_to_duration() {
        let ts = TimeSpan::from_seconds_f64(1.5);
        let duration = Duration::try_from(ts).unwrap();
        assert_eq!(duration.as_secs(), 1);
        assert_eq!(duration.subsec_millis(), 500);
    }

    #[test]
    fn test_instant_diff() {
        let earlier = std::time::Instant::now();
        let later = earlier + Duration::from_millis(100);
        let diff = TimeSpan::from_instant_diff(later, earlier);
        assert!(diff.is_positive());
        assert!(diff.total_milliseconds() >= 100.0);
        assert!(diff.total_milliseconds() < 101.0); // Should be close to 100ms
    }

    #[test]
    fn test_timespan_negation() {
        let ts = TimeSpan::from_seconds_f64(1.5);
        let neg_ts = -ts;
        assert!(neg_ts.is_negative());
        assert_eq!(neg_ts.total_seconds(), -1.5);
    }
}
