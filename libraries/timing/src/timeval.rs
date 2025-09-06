use crate::{TimeSpec, NSEC_PER_SEC, USEC_PER_SEC};
/// A time value structure representing time as seconds and microseconds.
/// 
/// This structure is compatible with the POSIX `timeval` structure and is 
/// commonly used in system programming for time representation with microsecond precision.
///
/// # Examples
///
/// ```
/// use timing::TimeVal;
///
/// // Create a TimeVal representing 1.5 seconds
/// let tv = TimeVal::new(1, 500_000);
/// assert_eq!(tv.total_seconds(), 1.5);
///
/// // Create using constructor methods
/// let zero = TimeVal::zero();
/// assert!(zero.is_zero());
///
/// // Arithmetic operations
/// let tv1 = TimeVal::new(2, 0);
/// let tv2 = TimeVal::new(1, 500_000);
/// let sum = tv1 + tv2;
/// assert_eq!(sum.total_seconds(), 3.5);
/// ```
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeVal {
    /// Seconds component of the time
    pub tv_sec: i64,
    /// Microseconds component of the time (0-999,999)
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

        // Handle overflow/underflow for microseconds
        if self.tv_msec >= USEC_PER_SEC {
            self.tv_sec += self.tv_msec / USEC_PER_SEC;
            self.tv_msec %= USEC_PER_SEC;
        } else if self.tv_msec < 0 {
            let borrow = (-self.tv_msec + USEC_PER_SEC - 1) / USEC_PER_SEC;
            self.tv_sec -= borrow;
            self.tv_msec += borrow * USEC_PER_SEC;
        }
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

    /// Get total microseconds as i64
    pub fn total_microseconds(&self) -> i64 {
        self.tv_sec * USEC_PER_SEC + self.tv_msec
    }

    /// Get total nanoseconds as i64
    pub fn total_nanoseconds(&self) -> i64 {
        self.tv_sec * NSEC_PER_SEC + self.tv_msec * 1_000
    }

    /// Check if this TimeVal is zero
    pub fn is_zero(&self) -> bool {
        self.tv_sec == 0 && self.tv_msec == 0
    }

    /// Check if this TimeVal is positive
    pub fn is_positive(&self) -> bool {
        self.tv_sec > 0 || (self.tv_sec == 0 && self.tv_msec > 0)
    }

    /// Check if this TimeVal is negative
    pub fn is_negative(&self) -> bool {
        self.tv_sec < 0 || (self.tv_sec == 0 && self.tv_msec < 0)
    }

    /// Get the absolute value of this TimeVal
    pub fn abs(&self) -> TimeVal {
        if self.is_negative() {
            TimeVal {
                tv_sec: -self.tv_sec,
                tv_msec: -self.tv_msec,
            }
        } else {
            *self
        }
    }

    /// Add seconds to this TimeVal
    pub fn add_seconds(&mut self, seconds: i64) {
        self.tv_sec += seconds;
    }

    /// Add milliseconds to this TimeVal
    pub fn add_milliseconds(&mut self, milliseconds: i64) {
        self.add_usec(milliseconds * 1_000);
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

#[cfg(test)]
mod test_timeval {
    use super::TimeVal;

    #[test]
    fn test_new() {
        let tv = TimeVal::new(10, 500_000);
        assert_eq!(tv.tv_sec, 10);
        assert_eq!(tv.tv_msec, 500_000);
    }

    #[test]
    fn test_zero() {
        let tv = TimeVal::zero();
        assert_eq!(tv.tv_sec, 0);
        assert_eq!(tv.tv_msec, 0);
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
        assert_eq!(tv.tv_msec, 0);

        let tv2 = TimeVal::from_ticks(1_500_000, 1_000_000); // 1.5 seconds at 1MHz  
        assert_eq!(tv2.tv_sec, 1);
        assert_eq!(tv2.tv_msec, 500_000);
    }

    #[test]
    fn test_add_usec() {
        let mut tv = TimeVal::new(1, 500_000);
        tv.add_usec(500_000);
        assert_eq!(tv.tv_sec, 2);
        assert_eq!(tv.tv_msec, 0);

        // Test negative addition
        let mut tv2 = TimeVal::new(2, 300_000);
        tv2.add_usec(-500_000);
        assert_eq!(tv2.tv_sec, 1);
        assert_eq!(tv2.tv_msec, 800_000);
    }

    #[test]
    fn test_add_usec_overflow() {
        let mut tv = TimeVal::new(0, 800_000);
        tv.add_usec(300_000);
        assert_eq!(tv.tv_sec, 1);
        assert_eq!(tv.tv_msec, 100_000);
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
        assert_eq!(result.tv_msec, 100_000);
    }

    #[test]
    fn test_add_assign_timeval() {
        let mut tv1 = TimeVal::new(1, 300_000);
        let tv2 = TimeVal::new(0, 800_000);
        tv1 += tv2;
        assert_eq!(tv1.tv_sec, 2);
        assert_eq!(tv1.tv_msec, 100_000);
    }

    #[test]
    fn test_sub_timeval() {
        let tv1 = TimeVal::new(3, 200_000);
        let tv2 = TimeVal::new(1, 800_000);
        let result = tv1 - tv2;
        assert_eq!(result.tv_sec, 1);
        assert_eq!(result.tv_msec, 400_000);
    }

    #[test]
    fn test_sub_assign_timeval() {
        let mut tv1 = TimeVal::new(3, 200_000);
        let tv2 = TimeVal::new(1, 800_000);
        tv1 -= tv2;
        assert_eq!(tv1.tv_sec, 1);
        assert_eq!(tv1.tv_msec, 400_000);
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
}
