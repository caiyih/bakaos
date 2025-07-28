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

cfg_if::cfg_if! {
    if #[cfg(target_arch = "riscv64")] {
        type MachineImpl = riscv64::MachineImpl;
    } else if #[cfg(target_arch = "loongarch64")] {
        type MachineImpl = loongarch64::MachineImpl;
    } else {
        compile_error!("No valid machine feature enabled");
    }
}

static MACHINE_IMPL: MachineImpl = MachineImpl::new();

#[inline(always)]
#[allow(unreachable_code)]
pub fn machine() -> &'static dyn IMachine {
    &MACHINE_IMPL
}
