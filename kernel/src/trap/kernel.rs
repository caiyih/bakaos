use core::arch::asm;

use log::{debug, trace};
use riscv::{
    interrupt::{
        supervisor::{Exception, Interrupt},
        Trap,
    },
    register::{scause, stval, stvec},
};

use crate::kernel;

pub fn set_kernel_trap_handler() {
    trace!("Set trap handler to kernel");
    unsafe { stvec::write(__on_kernel_trap as usize, stvec::TrapMode::Direct) };
}

#[naked]
#[no_mangle]
#[link_section = ".text.trampoline_kernel"]
unsafe extern "C" fn __on_kernel_trap() -> ! {
    // Consider kernel tarp handler as a function call
    // We only have to save the caller-saved registers
    // and we can just jump to the kernel_trap_handler
    asm!(
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
        options(noreturn)
    )
}

#[no_mangle]
fn kernel_trap_handler() {
    let scause = scause::read().cause();
    let stval = stval::read();

    debug!("[Kernel trap] [{:?}] stval: {:#x}", scause, stval);
    let kstat = kernel::get().stat();

    let scause = unsafe { core::mem::transmute::<_, Trap<Interrupt, Exception>>(scause) };
    match scause {
        Trap::Interrupt(interrupt) => match interrupt {
            Interrupt::SupervisorSoft => kstat.on_software_interrupt(),
            Interrupt::SupervisorTimer => kstat.on_timer_interrupt(),
            Interrupt::SupervisorExternal => kstat.on_external_interrupt(),
        },
        Trap::Exception(_) => kstat.on_kernel_exception(),
    };
}
