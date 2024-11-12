use core::panic::PanicInfo;

use crate::{legacy_println, system};

#[panic_handler]
#[no_mangle]
unsafe fn __kernel_panic(info: &PanicInfo) -> ! {
    // legacy_println!("[BAKA-OS] Kernel panicked for: ", info.);
    match info.message() {
        Some(msg) => legacy_println!("[BAKA-OS] Kernel panicked for: {}", msg),
        None => legacy_println!("[BAKA-OS] Kernel panicked for Unknown reason"),
    }

    match info.location() {
        Some(location) => {
            legacy_println!(
                "[BAKA-OS]     at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        None => legacy_println!("[BAKA-OS]     No location information available."),
    }

    match info.payload().downcast_ref::<&str>() {
        Some(s) => legacy_println!("[BAKA-OS]     Payload: {}", s),
        None => legacy_println!("[BAKA-OS]     No payload information available."),
    }

    legacy_println!("[BAKA-OS]     can unwind: {}", info.can_unwind());

    if info.can_unwind() {
        // use Unwinding after global allocator is set up
        // https://github.com/nbdd0121/unwinding
    }

    legacy_println!("[BAKA-OS] Hanging the system...");

    system::shutdown_failure();
}
