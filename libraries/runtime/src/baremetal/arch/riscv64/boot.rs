use core::arch::global_asm;

use crate::baremetal::bss::clear_bss;

global_asm!(include_str!("boot.S"));

fn boot_init() {
    clear_bss();

    // TODO: init memory allocator

    crate::baremetal::init();

    super::cpu::init();
}

#[unsafe(no_mangle)]
extern "C" fn rust_boot_main() -> ! {
    boot_init();

    unsafe extern "Rust" {
        fn rust_main_entry();
    }

    unsafe { rust_main_entry() }; // transfer control to the kernel

    // TODO: use shutdown
    loop {}
}
