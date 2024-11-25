use kernel::set_kernel_trap_handler;

mod interrupts;
mod kernel;
mod user;

use riscv::register::sstatus;
pub use user::{return_to_user, user_trap_handler};

pub fn init() {
    unsafe { sstatus::set_sum() };

    set_kernel_trap_handler();
}
