use core::sync::atomic::AtomicBool;
use platform_specific::legacy_println;

use unwinding::StackTraceWalker;

#[allow(unused)]
pub(crate) static SKIP_PANIC_FRAME: AtomicBool = AtomicBool::new(false);

#[allow(unused)]
static PANIC_NESTING: AtomicBool = AtomicBool::new(false);

// A workaround to disable panic_impl for host target
#[cfg(not(panic = "unwind"))]
#[no_mangle]
#[panic_handler]
unsafe fn rust_begin_unwind(info: &::core::panic::PanicInfo) -> ! {
    extern "Rust" {
        fn panic_handler(info: &::core::panic::PanicInfo) -> !;
    }

    panic_handler(info)
}

#[no_mangle]
#[linkage = "weak"]
unsafe extern "Rust" fn panic_handler(info: &::core::panic::PanicInfo) -> ! {
    if !PANIC_NESTING.load(core::sync::atomic::Ordering::Relaxed) {
        PANIC_NESTING.store(true, core::sync::atomic::Ordering::Relaxed);

        legacy_println!("[BAKA-OS] Kernel panicked for: {}", info.message());

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

        legacy_println!("[BAKA-OS]     Can unwind: {}", info.can_unwind());

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
                    )
                };

            frames += 1;

            true
        });

        legacy_println!("[BAKA-OS]     Note: The unwinder script will resolve all stack frames when the kernel shutdown.");
        legacy_println!("[BAKA-OS]           If it didn't, you can copy all the {} lines above(including the note) and paste it to the unwinder.", frames + 1);
    } else {
        legacy_println!(
            "[BAKA-OS] Kernel panicked while handling another panic: {}",
            info.message()
        );
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
        legacy_println!("[BAKA-OS]     This is a bug in the kernel.");
        legacy_println!("[BAKA-OS]     The kernel will now shutdown.");
    }

    crate::machine_shutdown(true)
}
