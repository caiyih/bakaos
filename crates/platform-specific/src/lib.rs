#![no_std]
#![feature(stmt_expr_attributes)]

#[allow(private_interfaces)]
#[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
pub type TaskTrapContext = context::DummyTaskContext;

#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(target_arch = "riscv64")]
pub type TaskTrapContext = riscv64::TaskTrapContext;

#[cfg(target_arch = "loongarch64")]
mod loongarch64;

#[cfg(target_arch = "loongarch64")]
pub use loongarch64::*;

#[cfg(target_arch = "loongarch64")]
pub type TaskTrapContext = loongarch64::TaskTrapContext;

mod generated;

mod context;
mod serial;
mod syscalls;
pub use context::ITaskContext;
pub use serial::*;
pub use syscalls::*;
