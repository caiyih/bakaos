use crate::{TimeVal, NSEC_PER_SEC};

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

    pub fn zero() -> TimeSpec {
        TimeSpec {
            tv_sec: 0,
            tv_nsec: 0,
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
