use sbi_rt::{system_reset, NoReason, Shutdown, SystemFailure};

/// # Safety
/// Calling this function will shutdown the machine.
///
/// This function will never return.
pub unsafe fn system_shutdown(failure: bool) -> ! {
    match failure {
        true => system_reset(Shutdown, SystemFailure),
        false => system_reset(Shutdown, NoReason),
    };

    loop {
        core::arch::asm!("wfi")
    }
}
