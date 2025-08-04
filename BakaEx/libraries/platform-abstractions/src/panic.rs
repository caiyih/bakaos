use core::sync::atomic::{AtomicBool, AtomicU32};
use platform_specific::legacy_println;

use unwinding::StackTraceWalker;

#[allow(unused)]
pub(crate) static SKIP_PANIC_FRAME: AtomicBool = AtomicBool::new(false);

#[allow(unused)]
static PANIC_NESTING_DEPTH: AtomicU32 = AtomicU32::new(0);

// A workaround to disable panic_impl for host target
#[cfg(not(panic = "unwind"))]
#[panic_handler]
unsafe fn _rust_begin_unwind(info: &::core::panic::PanicInfo) -> ! {
    extern "Rust" {
        fn panic_handler(info: &::core::panic::PanicInfo) -> !;
    }

    panic_handler(info)
}

#[no_mangle]
#[linkage = "weak"]
#[cfg(target_os = "none")]
unsafe extern "Rust" fn panic_handler(info: &::core::panic::PanicInfo) -> ! {
    let nesting_depth = PANIC_NESTING_DEPTH.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    let msg = info.message();
    legacy_println!("[BAKA-OS] Kernel panicked for: {msg}");

    if nesting_depth > 0 {
        legacy_println!("[BAKA-OS] Kernel panic recursion detected!");
    }

    if nesting_depth < 2 {
        match info.location() {
            Some(location) => {
                legacy_println!(
                    // Why this line number and column number is hex?
                    // You should ask why does the rust standard library's decimal formatting causes address misalignment exception.
                    // Dumb as fuck
                    "[BAKA-OS]     at {}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                );
            }
            None => legacy_println!("[BAKA-OS]     No location information available."),
        }

        legacy_println!("[BAKA-OS]     Can unwind: {}", info.can_unwind());

        if nesting_depth < 1 {
            let mut skip_frames = 2;

            if SKIP_PANIC_FRAME.load(core::sync::atomic::Ordering::Relaxed) {
                skip_frames += 1;
            }

            legacy_println!("[BAKA-OS]     Stack trace:");

            let mut frames = 0;

            StackTraceWalker::begin_unwind(skip_frames, |index, frame| {
                // PC implies the next instruction of function call
                match frame.pc() {
                Ok(pc) => legacy_println!(
                    "[BAKA-OS]     {:4} at: {:#018x} Frame pointer: {:#018x}",
                    index + 1,
                    pc,
                    frame.fp()
                ),
                Err(ra) => legacy_println!(
                    "[BAKA-OS]     {:4} Frame pointer: {:#018x} Unrecognize instruction, ra: {:#018x}",
                    index + 1,
                    frame.fp(),
                    ra
                ),
            };

                frames += 1;

                true
            });

            legacy_println!("[BAKA-OS]     Note: The unwinder script will resolve all stack frames when the kernel shutdown.");
            legacy_println!("[BAKA-OS]           If it didn't, you can copy all the {} lines above(including the note) and paste it to the unwinder.", frames + 1);
        }
    }

    crate::machine_shutdown(true)
}
