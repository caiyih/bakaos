#![no_std]
#![feature(stmt_expr_attributes)]

#[allow(private_interfaces)]
#[cfg(not(any(target_arch = "riscv64")))]
pub type TaskTrapContext = context::DummyTaskContext;

#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(target_arch = "riscv64")]
pub type TaskTrapContext = riscv64::TaskTrapContext;

mod context;
mod serial;
pub use context::ITaskContext;
pub use serial::*;
