use crate::{TimeSpec, NSEC_PER_SEC, USEC_PER_SEC};

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

    pub fn zero() -> TimeVal {
        TimeVal {
            tv_msec: 0,
            tv_sec: 0,
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

impl Default for TimeVal {
    fn default() -> Self {
        Self::zero()
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
