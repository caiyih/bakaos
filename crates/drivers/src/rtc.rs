use timing::{TimeSpan, TimeSpec, TimeVal};

use crate::machine;

static mut TIME_OFFSEST: TimeSpec = TimeSpec {
    tv_sec: 0,
    tv_nsec: 0,
};

pub fn initialize(rtc_offset: TimeSpec) {
    unsafe { TIME_OFFSEST = rtc_offset };
}

pub fn current_timespec() -> TimeSpec {
    let machine = machine();
    let ticks = machine.query_performance_counter() as i64;
    let freq = machine.query_performance_frequency();
    TimeSpec::from_ticks(ticks, freq) + unsafe { TIME_OFFSEST }
}

#[allow(static_mut_refs)]
pub fn current_timeval() -> TimeVal {
    let machine = machine();
    let ticks = machine.query_performance_counter() as i64;
    let freq = machine.query_performance_frequency();
    TimeVal::from_ticks(ticks, freq) + unsafe { TIME_OFFSEST.to_timeval() }
}

pub trait ITimer {
    #[allow(unused)]
    fn is_started(&self) -> bool;
    fn start(&mut self);
    fn set(&mut self);

    fn elapsed(&self) -> TimeSpan;
}

#[derive(Debug, Clone)]
pub struct UserTaskTimer {
    pub total: TimeSpan,
    pub start: Option<TimeSpec>,
}

impl Default for UserTaskTimer {
    fn default() -> Self {
        UserTaskTimer {
            total: TimeSpan::zero(),
            start: None,
        }
    }
}

impl ITimer for UserTaskTimer {
    fn start(&mut self) {
        debug_assert!(self.start.is_none());
        self.start = Some(current_timespec());
    }

    fn set(&mut self) {
        debug_assert!(self.start.is_some());
        let now = current_timespec();
        let start = unsafe { self.start.unwrap_unchecked() };
        self.start = None;
        self.total += TimeSpan::from_timespec_diff(&now, &start);
    }

    fn elapsed(&self) -> TimeSpan {
        match self.start {
            Some(start) => self.total + TimeSpan::from_timespec_diff(&current_timespec(), &start),
            None => self.total,
        }
    }

    fn is_started(&self) -> bool {
        self.start.is_some()
    }
}
