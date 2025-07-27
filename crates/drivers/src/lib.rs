#![no_std]
extern crate alloc;

mod machine;
pub use machine::IMachine;

mod block;
pub use block::*;

#[cfg(target_arch = "riscv64")]
mod riscv64;

#[allow(unused_imports)]
#[cfg(target_arch = "riscv64")]
use riscv64::*;

#[cfg(target_arch = "loongarch64")]
mod loongarch64;

#[allow(unused_imports)]
#[cfg(target_arch = "loongarch64")]
use loongarch64::*;

mod rtc;
pub use rtc::{current_timespec, current_timeval, ITimer, UserTaskTimer};

pub fn initialize_rtc() {
    let rtc_offset = machine().get_rtc_offset();
    rtc::initialize(rtc_offset);
}

#[inline(always)]
#[allow(unreachable_code)]
pub fn machine() -> &'static dyn IMachine {
    #[cfg(target_arch = "riscv64")]
    {
        #[cfg(feature = "virt")]
        return &riscv64::virt::VirtMachine;

        #[cfg(feature = "vf2")]
        return &riscv64::vf2::VF2Machine;
    }

    #[cfg(target_arch = "loongarch64")]
    {
        #[cfg(feature = "virt")]
        return &loongarch64::virt::VirtMachine;
    }

    panic!("No avaliable machine interface")
}
