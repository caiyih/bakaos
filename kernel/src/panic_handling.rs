use core::{arch::asm, panic::PanicInfo};

use crate::{legacy_println, system};

#[inline(always)]
pub fn sp() -> usize {
    let ptr;
    unsafe {
        asm!("mv {}, sp", out(reg) ptr);
    }
    ptr
}

#[inline(always)]
pub fn fp() -> usize {
    let ptr;
    unsafe {
        asm!("mv {}, fp", out(reg) ptr);
    }
    ptr
}

#[inline(always)]
pub fn lr() -> usize {
    let ptr;
    unsafe {
        asm!("mv {}, ra", out(reg) ptr);
    }
    ptr
}

#[no_mangle]
fn stack_trace() {
    extern "C" {
        fn __tmp_stack_top();
        fn stext();
        fn etext();
    }

    let mut pc = lr();
    let mut fp = fp();
    let mut depth = 0;

    legacy_println!(
        "[BAKA-OS]     Tmp stack top: {:#018x}",
        __tmp_stack_top as usize
    );
    legacy_println!("[BAKA-OS]     Stack pointer: {:#018x}", sp());
    legacy_println!("[BAKA-OS]     Stack trace:");
    while pc >= stext as usize && pc <= etext as usize && fp >= stext as usize && fp != 0
    // // TODO: fp should be lower than __tmp_stack_top
    // // But the kernel may have mutiple stacks
    // Unnecessary check. The two lines comment above seems to be wrong. But leave it here for now.
    // && fp < __tmp_stack_top as usize
    {
        legacy_println!(
            "[BAKA-OS]     {:4} at: {:#018x} Frame pointer: {:#018x}",
            depth,
            // PC implies the next instruction of function call
            // So we need to subtract 4 to get the real pc
            pc - 4,
            fp
        );

        depth += 1;

        fp = unsafe { *(fp as *const usize).offset(-2) };
        pc = unsafe { *(fp as *const usize).offset(-1) };
    }

    legacy_println!("[BAKA-OS]     Note: Higher traces are deeper. You can check symbol files for detailed info.");
    legacy_println!("[BAKA-OS]           Or you can copy all the {} lines above(including the note) and paste it to the unwinder.", depth + 1);
}

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

    legacy_println!("[BAKA-OS]     Can unwind: {}", info.can_unwind());

    if info.can_unwind() {
        stack_trace();
    }

    system::shutdown_failure();
}
