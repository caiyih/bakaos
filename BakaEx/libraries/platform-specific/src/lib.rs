#![no_std]
#![feature(naked_functions)]
#![feature(stmt_expr_attributes)]

#[cfg(not(any(target_arch = "riscv64", target_arch = "loongarch64")))]
pub type TaskTrapContext = context::TestTaskContext;

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

impl ITaskTrapContext for TaskTrapContext {
    fn copy_from(&mut self, other: &dyn ITaskTrapContext) {
        let other = other
            .downcast_ref::<TaskTrapContext>()
            .expect("The other trap context is not of type TaskTrapContext");

        *self = *other;
    }
    
    fn set_stack_top(&mut self, stack_top: usize) {
        self.set_stack_top_internal(stack_top);
    }
    
    fn set_return_value(&mut self, ret: usize) {
        self.set_return_value_internal(ret)
    }
}

impl Default for TaskTrapContext {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

#[rustfmt::skip]
mod generated;

mod context;
mod syscalls;

pub use context::ITaskContext;
pub use syscalls::*;
use trap_abstractions::ITaskTrapContext;

#[cfg(target_os = "none")]
mod serial;

#[cfg(target_os = "none")]
pub use serial::*;

#[cfg(target_os = "none")]
unsafe extern "C" {
    pub unsafe fn __sigreturn_trampoline() -> !;
}
