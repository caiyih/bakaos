mod boot;
mod system;

use alloc::sync::Arc;
pub use boot::_start;
pub use system::{machine_shutdown, print_bootloader_info};

use crate::UserInterrupt;
use tasks::TaskControlBlock;

pub fn init_trap() {
    // TODO: this is a stub
}

pub fn translate_current_trap() -> UserInterrupt {
    // TODO: this is a stub
    UserInterrupt::Breakpoint
}

pub fn return_to_user(tcb: &Arc<TaskControlBlock>) {
    // TODO: this is a stub
}
