use core::arch::asm;

use crate::{
    ci_helper::{self, IQemuExitHandle},
    legacy_println,
};

#[allow(unreachable_code)]
pub fn shutdown_successfully() -> ! {
    legacy_println!("Shutting down with success...");

    #[cfg(feature = "virt")]
    {
        if ci_helper::is_ci_environment() {
            ci_helper::exit_qemu_successfully();
        } else {
            ci_helper::QEMU_EXIT_HANDLE.exit_success();
        }
    }

    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

#[allow(unreachable_code)]
pub fn shutdown_failure() -> ! {
    legacy_println!("Shutting down with failure...");

    #[cfg(feature = "virt")]
    {
        if ci_helper::is_ci_environment() {
            ci_helper::exit_qemu_failure();
        } else {
            ci_helper::QEMU_EXIT_HANDLE.exit_success();
        }
    }

    loop {
        unsafe {
            asm!("wfi");
        }
    }
}
