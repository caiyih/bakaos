#![no_std]
#![feature(naked_functions)]
#![feature(panic_can_unwind)]

extern crate alloc;

mod interrupts;
mod panic;
mod syscalls;

#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(target_arch = "riscv64")]
pub type SyscallContext = riscv64::RISCV64SyscallContext;

#[cfg(target_arch = "loongarch64")]
mod loongarch64;

#[cfg(target_arch = "loongarch64")]
pub use loongarch64::*;

pub use interrupts::*;
pub use syscalls::*;
