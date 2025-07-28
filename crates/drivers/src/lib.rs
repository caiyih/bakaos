#![no_std]
extern crate alloc;

mod machine;
pub use machine::IMachine;

mod block;
pub use block::*;

#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "loongarch64")]
mod loongarch64;

mod rtc;
pub use rtc::{current_timespec, current_timeval, ITimer, UserTaskTimer};

#[rustfmt::skip]
mod generated;
pub use generated::*;

pub fn initialize_rtc() {
    platform_specific::legacy_println!("Initializing RTC...");
    let machine = machine();
    let rtc_offset = machine.get_rtc_offset();
    rtc::initialize(rtc_offset);
}
