use core::arch::naked_asm;

use log::debug;
use riscv::{
    interrupt::{
        supervisor::{Exception, Interrupt},
        Trap,
    },
    register::{
        scause, sepc, stval,
        stvec::{self},
    },
};

use crate::panic::SKIP_PANIC_FRAME;

pub fn set_kernel_trap_handler() {
    unsafe { stvec::write(__on_kernel_trap as usize, stvec::TrapMode::Direct) };
}

#[naked]
#[no_mangle]
#[link_section = ".text.trampoline_kernel"]
unsafe extern "C" fn __on_kernel_trap() {
    // Consider kernel tarp handler as a function call
    // We only have to save the caller-saved registers
    // and we can just jump to the kernel_trap_handler
    naked_asm!(
        // Allocate space for the kernel trap context
        "addi sp, sp, -17*8",
        // Save the caller-saved registers
        "sd  ra, 1*8(sp)", // we are saving on the stack, so the first element is 1*8
        "sd  t0,  2*8(sp)",
        "sd  t1,  3*8(sp)",
        "sd  t2,  4*8(sp)",
        "sd  t3,  5*8(sp)",
        "sd  t4,  6*8(sp)",
        "sd  t5,  7*8(sp)",
        "sd  t6,  8*8(sp)",
        "sd  a0,  9*8(sp)",
        "sd  a1, 10*8(sp)",
        "sd  a2, 11*8(sp)",
        "sd  a3, 12*8(sp)",
        "sd  a4, 13*8(sp)",
        "sd  a5, 14*8(sp)",
        "sd  a6, 15*8(sp)",
        "sd  a7, 16*8(sp)",
        // Enter the kernel_trap_handler
        "call kernel_trap_handler",
        // Back from the kernel_trap_handler, restore the caller-saved registers
        "ld  ra, 1*8(sp)",
        "ld  t0,  2*8(sp)",
        "ld  t1,  3*8(sp)",
        "ld  t2,  4*8(sp)",
        "ld  t3,  5*8(sp)",
        "ld  t4,  6*8(sp)",
        "ld  t5,  7*8(sp)",
        "ld  t6,  8*8(sp)",
        "ld  a0,  9*8(sp)",
        "ld  a1, 10*8(sp)",
        "ld  a2, 11*8(sp)",
        "ld  a3, 12*8(sp)",
        "ld  a4, 13*8(sp)",
        "ld  a5, 14*8(sp)",
        "ld  a6, 15*8(sp)",
        "ld  a7, 16*8(sp)",
        "addi sp, sp, 17*8", // Clear the space for the kernel trap context
        "sret",
    )
}

#[no_mangle]
fn kernel_trap_handler() {
    let scause = scause::read().cause();
    let stval = stval::read();

    let scause =
        unsafe { core::mem::transmute::<Trap<usize, usize>, Trap<Interrupt, Exception>>(scause) };

    debug!(
        "[Kernel trap] [{:?}] stval: {:#x}, sepc: {:#018x}",
        scause,
        stval,
        sepc::read()
    );

    match scause {
        Trap::Interrupt(_) => (),
        Trap::Exception(e) => unsafe { __unhandled_kernel_exception(e, stval) },
    };
}

#[inline(always)]
unsafe fn __unhandled_kernel_exception(e: Exception, stval: usize) -> ! {
    use ::core::ptr::NonNull;

    let sepc = sepc::read();

    if let Ok(pc_size) = platform_specific::get_instruction_size(sepc) {
        let sra = sepc + pc_size;

        let fp = platform_specific::fp();
        if let Some(p_ra_1) = NonNull::new((fp - core::mem::size_of::<usize>()) as *mut usize) {
            p_ra_1.write_volatile(sra);

            SKIP_PANIC_FRAME.store(true, core::sync::atomic::Ordering::Relaxed);

            panic!(
                "Unhandled Supervisor exception: {:?}, stval: {:#018x}",
                e, stval
            )
        }
    }

    panic!(
        "Unhandled Supervisor exception: {:?} at: {:#018x} stval: {:#018x}. Unable to generate trace frame, please unwind it manually",
        e, sepc, stval
    )
}
