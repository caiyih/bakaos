use core::{panic::PanicInfo, sync::atomic::AtomicBool};

use unwinding::StackTrace;

use crate::{legacy_println, system};

static mut PANIC_NESTING: AtomicBool = AtomicBool::new(false);

#[panic_handler]
#[no_mangle]
unsafe fn rust_begin_unwind(info: &PanicInfo) -> ! {
    if !PANIC_NESTING.load(core::sync::atomic::Ordering::Relaxed) {
        PANIC_NESTING.store(true, core::sync::atomic::Ordering::Relaxed);

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

        legacy_println!("[BAKA-OS]     Can unwind: {}", info.can_unwind());

        if info.can_unwind() {
            StackTrace::begin_unwind(1).print_trace();
        }
    } else {
        legacy_println!("[BAKA-OS] Kernel panicked while handling another panic.");
        legacy_println!("[BAKA-OS]     This is a bug in the kernel.");
        legacy_println!("[BAKA-OS]     The kernel will now shutdown.");
    }

    system::shutdown_failure();
}

pub trait IDisplayableStackTrace {
    fn print_trace(&self);
}

impl IDisplayableStackTrace for StackTrace {
    fn print_trace(&self) {
        let frames = self.stack_frames();

        legacy_println!("[BAKA-OS]     Stack trace:");

        for (depth, frame) in frames.iter().enumerate() {
            let ra = frame.ra();

            // PC implies the next instruction of function call
            match unwinding::find_previous_instruction(ra) {
                Ok(pc) => legacy_println!(
                    "[BAKA-OS]     {:4} at: {:#018x} Frame pointer: {:#018x}",
                    depth + 1,
                    pc,
                    frame.fp()
                ),
                Err(ins64) => legacy_println!(
                    "[BAKA-OS]     {:4} Frame pointer: {:#018x} Unrecognize instruction, ra: {:#018x}, instruction 64bits: {:#018x}",
                    depth + 1,
                    frame.fp(),
                    ra,
                    ins64
                ),
            }
        }

        legacy_println!("[BAKA-OS]     Note: The unwinder script will resolve all stack frames when the kernel shutdown.");
        legacy_println!("[BAKA-OS]           If it didn't, you can copy all the {} lines above(including the note) and paste it to the unwinder.", frames.len() + 1);
    }
}
