use core::arch::asm;
use riscv::register::stvec;

mod kernel;
mod user;
mod interrupts;

fn set_kernel_trap_handler() {
    unsafe { stvec::write(__on_kernel_trap as usize, stvec::TrapMode::Direct) };
}

fn set_user_trap_handler() {
    unsafe { stvec::write(__on_user_trap as usize, stvec::TrapMode::Direct) };
}

pub fn init() {
    set_kernel_trap_handler();
}

#[naked]
#[no_mangle]
#[link_section = ".text.trampoline"]
unsafe extern "C" fn __on_user_trap() -> ! {
    asm!("unimp", options(noreturn));
}

#[naked]
#[no_mangle]
#[link_section = ".text.trampoline"]
unsafe extern "C" fn __return_from_user_trap() -> ! {
    asm!("unimp", options(noreturn));
}

#[naked]
#[no_mangle]
#[link_section = ".text.trampoline"]
unsafe extern "C" fn __on_kernel_trap() -> ! {
    asm!("unimp", options(noreturn));
}

#[naked]
#[no_mangle]
#[link_section = ".text.trampoline"]
unsafe extern "C" fn __return_from_kernel_trap() -> ! {
    asm!("unimp", options(noreturn));
}
