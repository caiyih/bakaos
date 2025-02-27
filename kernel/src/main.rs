// Please set the workspace to the kernel directory
// You will not gain in-vscode debug feature if you set the workspace to the root directory
#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(panic_can_unwind)]
#![feature(alloc_error_handler)]
#![allow(internal_features)]
#![feature(core_intrinsics)]

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
mod shared_memory;
mod statistics;
mod syscalls;
mod system;
mod timing;
mod trap;

use alloc::string::String;
use core::{arch::naked_asm, sync::atomic::AtomicBool};
use filesystem_abstractions::{global_mount_inode, global_open};
use firmwares::console::{IConsole, KernelMessageInode};
use paging::PageTable;
use sbi_spec::base::impl_id;
use scheduling::ProcDeviceInode;
use tasks::ProcessControlBlock;

extern crate alloc;

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    naked_asm!(
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
    )
}

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start_virtualized() -> ! {
    naked_asm!(
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
fn main() {
    match option_env!("KERNEL_TEST") {
        Some(profile) => match profile.chars().next().unwrap_or('\0').to_ascii_uppercase() {
            'F' => run_final_tests(),
            'P' => run_preliminary_tests(),
            _ => panic!("Unrecognized kernel test profile: {}", profile),
        },
        None => run_preliminary_tests(),
    }
}

fn setup_common_tools() {
    let busybox = global_open("/mnt/busybox", None).unwrap();
    let bin = global_open("/bin", None).unwrap();

    for tool in [
        "sh", "cp", "ls", "mv", "cat", "mkdir", "pwd", "rm", "grep", "busybox",
    ] {
        bin.hard_link(tool, &busybox).unwrap();
    }
}

#[allow(unused)]
fn run_final_tests() {
    use filesystem_abstractions::IFileSystem;
    use paging::MemorySpaceBuilder;
    use scheduling::spawn_task;
    use tasks::TaskControlBlock;

    setup_common_tools();

    let script = global_open("/", None)
        .unwrap()
        .touch("test_script.sh")
        .unwrap();
    script.writeat(0, include_bytes!("test_script.sh")).unwrap();

    run_busybox(
        "/mnt/busybox",
        &["sh", "/test_script.sh"],
        &[
            "HOME=/root",
            "PATH=/mnt:/bin",
            "USER=cirno",
            "LOGNAME=cirno",
            "TERM=xterm-256color",
            "PWD=/mnt",
            "SHELL=/bin/sh",
            "SHLVL=1",
            "LANG=C",
        ],
    );

    fn run_busybox(path: &str, args: &[&str], envp: &[&str]) {
        let busybox = filesystem_abstractions::global_open(path, None).unwrap();
        let busybox = busybox.readall().unwrap();

        let mut memspace = MemorySpaceBuilder::from_elf(&busybox, path).unwrap();

        drop(busybox);

        memspace.init_stack(args, envp);
        let task = ProcessControlBlock::new(memspace);
        unsafe {
            task.pcb.lock().cwd = String::from("/mnt");
        };

        spawn_task(task);
        threading::run_tasks();
    }
}

#[allow(unused)]
fn run_preliminary_tests() {
    fn preliminary_test(path: &str, args: Option<&[&str]>, envp: Option<&[&str]>) {
        use paging::MemorySpaceBuilder;
        use scheduling::spawn_task;
        use tasks::TaskControlBlock;

        let mut memspace = {
            let elf = filesystem_abstractions::global_open(path, None)
                .expect("Failed to open path")
                .readall()
                .expect("Failed to read file");

            MemorySpaceBuilder::from_elf(&elf, path).unwrap()
        };

        memspace.init_stack(args.unwrap_or(&[]), envp.unwrap_or(&[]));
        let task = ProcessControlBlock::new(memspace);
        unsafe {
            let directory = path::get_directory_name(path).unwrap();
            task.pcb.lock().cwd = String::from(directory);
        };
        spawn_task(task);
        threading::run_tasks();
    }

    // mount and umount tests requires '/dev/vda2'.
    // so we just use a copy of the sdcard's block device
    let sdcard = global_open("/dev/sda", None).unwrap();
    filesystem_abstractions::global_mount(&sdcard, "/dev/vda2", None).unwrap();

    preliminary_test("/mnt/uname", None, None);
    preliminary_test("/mnt/write", None, None);
    preliminary_test("/mnt/times", None, None);
    preliminary_test("/mnt/brk", None, None);
    preliminary_test("/mnt/gettimeofday", None, None);
    preliminary_test("/mnt/getpid", None, None);
    preliminary_test("/mnt/getppid", None, None);
    preliminary_test("/mnt/getcwd", None, None);
    preliminary_test("/mnt/sleep", None, None);
    preliminary_test("/mnt/fork", None, None);
    preliminary_test("/mnt/clone", None, None);
    preliminary_test("/mnt/yield", None, None);
    preliminary_test("/mnt/exit", None, None);
    preliminary_test("/mnt/wait", None, None);
    preliminary_test("/mnt/waitpid", None, None);
    preliminary_test("/mnt/execve", None, None);
    preliminary_test("/mnt/pipe", None, None);
    preliminary_test("/mnt/dup", None, None);
    preliminary_test("/mnt/dup2", None, None);
    preliminary_test("/mnt/openat", None, None);
    preliminary_test("/mnt/open", None, None);
    preliminary_test("/mnt/close", None, None);
    preliminary_test("/mnt/read", None, None);
    preliminary_test("/mnt/mount", None, None);
    preliminary_test("/mnt/umount", None, None);
    preliminary_test("/mnt/mkdir_", None, None);
    preliminary_test("/mnt/chdir", None, None);
    preliminary_test("/mnt/fstat", None, None);
    preliminary_test("/mnt/getdents", None, None);
    preliminary_test("/mnt/unlink", None, None);
    preliminary_test("/mnt/mmap", None, None);
    preliminary_test("/mnt/munmap", None, None);
}

static BOOTED: AtomicBool = AtomicBool::new(false);

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

    processor::init_processor_pool();

    BOOTED.store(true, core::sync::atomic::Ordering::Relaxed);

    filesystem_abstractions::initialize();
    ProcDeviceInode::setup();

    let sda = machine.create_block_device_at(0);
    filesystem_abstractions::global_mount_inode(&sda, "/dev/sda", None).unwrap();

    filesystem::global_mount_device("/dev/sda", "/mnt", None).unwrap();

    let etc = global_open("/etc", None).unwrap();
    let passwd = etc.touch("passwd").unwrap();
    passwd.writeat(0, b"cirno:x:0:0::/root:/bin/bash").unwrap();

    let kmsg = KernelMessageInode::new();
    global_mount_inode(&kmsg, "/dev/kmsg", None).unwrap();
    global_mount_inode(&kmsg, "/proc/kmsg", None).unwrap();

    let rtc_time = display_current_time(8);

    let seed = (((rtc_time.tv_nsec as u64) << 32) | machine.clock_freq()) ^ 0xdeadbeef;

    log::info!("Setting up global rng with seed: {}", seed);

    rng::initialize(seed);
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
    core::ptr::write_bytes(begin as *mut u8, 0, end - begin);
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

fn display_current_time(timezone_offset: i64) -> TimeSpec {
    #[inline(always)]
    fn is_leap_year(year: i64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    #[inline(always)]
    fn days_in_month(year: i64, month: u8) -> u8 {
        const DAYS: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        if month == 2 && is_leap_year(year) {
            29
        } else {
            DAYS[(month - 1) as usize]
        }
    }

    let time_spec = crate::timing::current_timespec();

    let mut total_seconds = time_spec.tv_sec + timezone_offset * 3600;

    let seconds = (total_seconds % 60) as u8;
    total_seconds /= 60;
    let minutes = (total_seconds % 60) as u8;
    total_seconds /= 60;
    let hours = (total_seconds % 24) as u8;
    total_seconds /= 24;

    let mut year = 1970;
    while total_seconds >= if is_leap_year(year) { 366 } else { 365 } {
        total_seconds -= if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }

    let mut month = 1;
    while total_seconds >= days_in_month(year, month) as i64 {
        total_seconds -= days_in_month(year, month) as i64;
        month += 1;
    }

    let day = (total_seconds + 1) as u8;

    log::info!(
        "Welcome, current time is: {:04}-{:02}-{:02} {:02}:{:02}:{:02}(UTC+{:02})",
        year,
        month,
        day,
        hours,
        minutes,
        seconds,
        timezone_offset
    );

    time_spec
}
