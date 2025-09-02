use kernel::set_kernel_trap_handler;

mod kernel;
mod user;

use riscv::register::sstatus;
pub use user::*;

pub fn init() {
    unsafe { sstatus::set_sum() };

    set_kernel_trap_handler();
}
