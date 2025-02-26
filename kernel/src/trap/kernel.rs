use core::arch::naked_asm;

use log::debug;
use riscv::{
    interrupt::{
        supervisor::{Exception, Interrupt},
        Trap,
    },
    register::{scause, sepc, stval, stvec},
};

use crate::kernel;

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

    let kstat = kernel::get().stat();

    let scause =
        unsafe { core::mem::transmute::<Trap<usize, usize>, Trap<Interrupt, Exception>>(scause) };

    debug!(
        "[Kernel trap] [{:?}] stval: {:#x}, sepc: {:#018x}",
        scause,
        stval,
        sepc::read()
    );

    match scause {
        Trap::Interrupt(interrupt) => match interrupt {
            Interrupt::SupervisorSoft => kstat.on_software_interrupt(),
            Interrupt::SupervisorTimer => kstat.on_timer_interrupt(),
            Interrupt::SupervisorExternal => kstat.on_external_interrupt(),
        },
        Trap::Exception(e) => {
            kstat.on_kernel_exception();

            unsafe { __unhandled_kernel_exception(e) }
        }
    };
}

#[inline(always)]
unsafe fn __unhandled_kernel_exception(e: Exception) -> ! {
    #[cfg(target_arch = "riscv64")]
    __rv64_unhandled_kernel_exception_construct_frame();

    panic!("Unhandled Supervisor exception: {:?}", e);
}

#[inline(always)]
unsafe fn __rv64_unhandled_kernel_exception_construct_frame() {
    use ::core::ptr::NonNull;

    let sepc = sepc::read();

    if let Ok(pc_size) = unwinding::get_instruction_size(sepc) {
        let sra = sepc + pc_size;

        let fp = unwinding::fp();
        if let Some(p_ra_1) = NonNull::new((fp - core::mem::size_of::<usize>()) as *mut usize) {
            p_ra_1.write_volatile(sra);
        }
    }
}
