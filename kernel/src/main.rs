// Please set the workspace to the kernel directory
// You will not gain in-vscode debug feature if you set the workspace to the root directory
#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(panic_can_unwind)]
#![feature(inline_const)]
#![feature(alloc_error_handler)]
#![feature(asm_const)]

mod ci_helper;
mod firmwares;
mod kernel;
mod logging;
mod memory;
mod panic_handling;
mod platform;
mod processor;
mod scheduling;
mod serial;
mod statistics;
mod syscalls;
mod system;
mod timing;
mod trap;

use core::{arch::asm, sync::atomic::AtomicBool};
use firmwares::console::IConsole;
use paging::{MemorySpaceBuilder, PageTable};
use sbi_spec::base::impl_id;
use scheduling::spawn_task;
use tasks::TaskControlBlock;

extern crate alloc;

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    asm!(
        // Read the hart id
        "mv tp, a0",
        // Read the device tree address
        "mv gp, a1",
        // Setup virtual memory
        // See comments below for details
        "la t0, {page_table}",
        "srli t0, t0, 12", // get the physical page number of PageTabe
        "li t1, 8 << 60",
        "or t0, t0, t1", // ppn | 8 << 60
        "csrw satp, t0",
        "sfence.vma",
        // jump to virtualized entry
        "li t1, {virt_addr_offset}",
        "la t0, {entry}",
        "or t0, t0, t1",
        // Do not save the return address to ra
        "jr t0",
        page_table = sym PAGE_TABLE,
        virt_addr_offset = const constants::VIRT_ADDR_OFFSET,
        entry = sym _start_virtualized,
        options(noreturn)
    )
}

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start_virtualized() -> ! {
    asm!(
        // Naver come back!
        "xor ra, ra, ra",
        // Clear fp so that unwind knows where to stop
        "xor fp, fp, fp",
        // Load the stack pointer after we entered the high half
        // The symbols are loaded with a fixed offset to PC
        // If we load the stack pointer before we entered the high half
        // The stack pointer will be in the low half, which is not what we want
        // But I still `or` the stack pointer with the offset to make the code more readable
        "la sp, __tmp_stack_top",
        "li t0, {virt_addr_offset}",
        "or sp, t0, sp",
        "j __kernel_start_main",
        virt_addr_offset = const constants::VIRT_ADDR_OFFSET,
        options(noreturn)
    )
}

// This basically includes two parts
//   1. Identity mapping of [0x40000000, 0x80000000) and [0x80000000, 0xc0000000)]
//   2. High half kernel mapping of
//      [ VIRTUAL_ADDRESS_OFFSET | 0x00000000, VIRTUAL_ADDRESS_OFFSET | 0x40000000)
//           to [0x00000000, 0x40000000)
//
//      [ VIRTUAL_ADDRESS_OFFSET | 0x40000000, VIRTUAL_ADDRESS_OFFSET | 0x80000000)
//           to [0x40000000, 0x80000000)
//
//      [ VIRTUAL_ADDRESS_OFFSET | 0x80000000, VIRTUAL_ADDRESS_OFFSET | 0xc0000000)
//           to [0x80000000, 0xc0000000)
//
// The first part is essential as the pc is still at the low half
// since satp is write until jump to virtualized entry
// But the two pages is not needed after the kernel entered the _start_virtualized
#[link_section = ".data.prepage"]
static mut PAGE_TABLE: [usize; 512] = {
    let mut arr: [usize; 512] = [0; 512];
    arr[1] = (0x40000 << 10) | 0xcf;
    arr[2] = (0x80000 << 10) | 0xcf;
    // Should be '(0x00000 << 10) | 0xcf' for clarifity
    // But Cargo clippy complains about this line, so i just write 0xcf here
    arr[0x100] = 0xcf;
    arr[0x101] = (0x40000 << 10) | 0xcf;
    arr[0x102] = (0x80000 << 10) | 0xcf;
    arr
};

#[no_mangle]
#[allow(unused_assignments)]
fn main() {
    preliminary_test("/uname", None, None);
    preliminary_test("/write", None, None);
    preliminary_test("/times", None, None);
    preliminary_test("/brk", None, None);
    preliminary_test("/gettimeofday", None, None);
    preliminary_test("/getpid", None, None);
    preliminary_test("/getppid", None, None);
    preliminary_test("/getcwd", None, None);
}

fn preliminary_test(path: &str, args: Option<&[&str]>, envp: Option<&[&str]>) {
    let elf = filesystem::root_filesystem()
        .lookup(path)
        .expect("Failed to open path")
        .readall()
        .expect("Failed to read file");

    let mut memspace = MemorySpaceBuilder::from_elf(&elf).unwrap();
    memspace.init_stack(args.unwrap_or(&[]), envp.unwrap_or(&[]));
    let task = TaskControlBlock::new(memspace);
    unsafe { task.cwd.get().as_mut().unwrap().push('/'); }; // SD card is mounted at root
    spawn_task(task);
    threading::run_tasks();
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

    let machine = kernel::get().machine();
    allocation::init(machine.memory_end());

    // Must be called after allocation::init because it depends on frame allocator
    paging::init(PageTable::borrow_current());

    filesystem::setup_root_filesystem(machine.create_fat32_filesystem_at_bus(0));

    processor::init_processor_pool();

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

    // After benchmarking, we got results below:
    // clear_bss_for_loop:
    //    ~160 ticks            iter 0
    //    ~40 ticks             iter 1 to 20
    // clear_bss_fast:
    //    ~203 ticks            iter 0
    //    ~2 ticks              iter 1 to 20
    // clear_bss_slice_fill:
    //    ~470 ticks            iter 0
    //    ~9 ticks              iter 1 to 20
    // We can see that clear_bss_for_loop is the fastest at the first iteration
    // Although clear_bss_fast is MUCH FASTER at the following iterations than it
    // Since We only have to clear bss once, we choose clear_bss_for_loop
    // This may be related to the CPU cache and branch prediction
    // because only the first iteration is affected the most
    // Also, we use u64 to write memory, which is faster than u8
    // And the compiler will actually unroll the loop by 2 times
    // So the actual loop writes 128 bits at a time
    clear_bss_for_loop(sbss as usize, ebss as usize);
}

unsafe fn clear_bss_for_loop(begin: usize, end: usize) {
    let mut ptr = begin as *mut u64;

    // The compiler unrolls the loop by 2 times, generating asm like below
    // while ptr < end {
    //     sd x0, 0(ptr)
    //     sd x0, 8(ptr)
    //     addi ptr, ptr, 16
    // }
    while (ptr as usize) < end {
        ptr.write_volatile(0);
        ptr = ptr.add(1);
    }
}

// This method is no longer used
// See comments in clear_bss for details
// unsafe fn clear_bss_fast(mut begin: usize, end: usize) {
//     // bss sections must be 4K aligned
//     debug_assert!(begin & 4095 == 0);
//     debug_assert!(end & 4095 == 0);
//     debug_assert!((end - begin) & 4095 == 0);

//     // Since riscv64gc supports neither SIMD or 128 bit integer operations
//     // We can only uses unsigned 64 bit integers to write memory
//     // u64 writes 64 bits at a time，still faster than u8 writes
//     // let mut ptr = begin as *mut u64;

//     // 8 times loop unrolling
//     // since the bss section is 4K aligned, we can safely write 512 bits at a time
//     while begin < end {
//         asm!(
//             "sd x0, 0({0})",
//             "sd x0, 8({0})",
//             "sd x0, 16({0})",
//             "sd x0, 24({0})",
//             "sd x0, 32({0})",
//             "sd x0, 40({0})",
//             "sd x0, 48({0})",
//             "sd x0, 56({0})",
//             in(reg) begin
//         );

//         begin += 16;
//     }
// }
