// Please set the workspace to the kernel directory
// You will not gain in-vscode debug feature if you set the workspace to the root directory
#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(panic_can_unwind)]
#![feature(inline_const)]
#![feature(alloc_error_handler)]

mod ci_helper;
mod firmwares;
mod kernel;
mod logging;
mod memory;
mod panic_handling;
mod platform;
mod serial;
mod statistics;
mod system;
mod timing;
mod trap;

use core::{arch::asm, sync::atomic::AtomicBool};
use firmwares::console::IConsole;
use sbi_spec::base::impl_id;

#[no_mangle]
#[allow(unused_assignments)]
fn main() {}

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    asm!(
        // Read the hart id
        "mv tp, a0",
        // Read the device tree address
        "mv gp, a1",
        "xor fp, fp, fp",
        "la sp, __tmp_stack_top",
        "j __kernel_start_main",
        options(noreturn)
    )
}

static mut BOOTED: AtomicBool = AtomicBool::new(false);

#[no_mangle]
#[allow(named_asm_labels)]
unsafe extern "C" fn __kernel_init() {
    if BOOTED.load(core::sync::atomic::Ordering::Relaxed) {
        // TODO: non-main harts should wait for main hart to finish booting
        // Setup non-main hart's temporary stack
        return;
    }

    clear_bss();
    debug_info();
    logging::init();
    kernel::init();

    memory::init();

    BOOTED.store(true, core::sync::atomic::Ordering::Relaxed);
}

#[no_mangle]
#[link_section = ".text.entry"]
#[allow(named_asm_labels)]
unsafe extern "C" fn __kernel_start_main() -> ! {
    __kernel_init();

    // TODO: Setup interrupt/trap subsystem
    trap::init();

    main();

    system::shutdown_successfully();
}

fn debug_info() {
    legacy_println!("Welcome to BAKA OS!");

    legacy_println!("SBI specification version: {0}", sbi_rt::get_spec_version());

    let sbi_impl = sbi_rt::get_sbi_impl_id();
    let sbi_impl = match sbi_impl {
        impl_id::BBL => "Berkley Bootloader",
        impl_id::OPEN_SBI => "OpenSBI",
        impl_id::XVISOR => "Xvisor",
        impl_id::KVM => "Kvm",
        impl_id::RUST_SBI => "RustSBI",
        impl_id::DIOSIX => "Diosix",
        impl_id::COFFER => "Coffer",
        _ => "Unknown",
    };

    legacy_println!("SBI implementation: {0}", sbi_impl);

    legacy_println!("Console type: {0}", serial::legacy_console().name());
}

unsafe fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }

    clear_bss_fast(sbss as usize, ebss as usize);
}

unsafe fn clear_bss_fast(begin: usize, end: usize) {
    // bss sections must be 4K aligned
    debug_assert!(begin & 4095 == 0);
    debug_assert!(end & 4095 == 0);
    debug_assert!((end - begin) & 4095 == 0);

    // Since riscv64gc supports neither SIMD or 128 bit integer operations
    // We can only uses unsigned 64 bit integers to write memory
    // u64 writes 64 bits at a time，still faster than u8 writes
    let mut ptr = begin as *mut u64;

    // 8 times loop unrolling
    // since the bss section is 4K aligned, we can safely write 512 bits at a time
    while (ptr as usize) < end {
        asm!(
            "sd x0, 0({0})",
            "sd x0, 8({0})",
            "sd x0, 16({0})",
            "sd x0, 24({0})",
            "sd x0, 32({0})",
            "sd x0, 40({0})",
            "sd x0, 48({0})",
            "sd x0, 56({0})",
            in(reg) ptr
        );

        ptr = ptr.add(8);
    }
}
