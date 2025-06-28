// Please set the workspace to the kernel directory
// You will not gain in-vscode debug feature if you set the workspace to the root directory
#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(panic_can_unwind)]
#![feature(alloc_error_handler)]
#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(cfg_accessible)]

mod dmesg;
mod kernel;
mod logging;
mod memory;
mod processor;
mod scheduling;
mod shared_memory;
mod statistics;
mod syscalls;
mod trap;

use address::{IConvertableVirtualAddress, VirtualAddress};
use alloc::{string::String, sync::Arc};
use core::sync::atomic::AtomicBool;
use dmesg::KernelMessageInode;
use drivers::current_timespec;
use filesystem_abstractions::{global_mount_inode, global_open, IInode};
use paging::PageTable;
use platform_specific::legacy_println;
use scheduling::ProcDeviceInode;
use tasks::ProcessControlBlock;

extern crate alloc;

// Rust compiler leaves out `_start` if not explicitly used in the exact project to be compiled
// So we use a stub to ensure it compiled.
// This function does not have to be called, and is even not compiled
// it's only used to cheat the compiler
unsafe extern "C" fn __ensure_start_compiled() -> ! {
    platform_abstractions::_start();
}

#[cfg(target_arch = "loongarch64")]
macro_rules! libc_val {
    () => {
        "glibc"
    };
}

#[cfg(not(target_arch = "loongarch64"))]
macro_rules! libc_val {
    () => {
        "musl"
    };
}

macro_rules! libc_spec {
    ($left:literal, $right:literal) => {
        concat!($left, libc_val!(), $right)
    };
}

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
    let busybox = global_open(libc_spec!("/mnt/", "/busybox"), None).unwrap();
    let bin = global_open("/bin", None).unwrap();

    for tool in [
        "sh", "cp", "ls", "mv", "cat", "mkdir", "pwd", "rm", "grep", "busybox", "vi", "clear",
        "touch", "uname", "whoami", "arch", "date",
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

    // mount and umount tests requires a node at '/dev/vda2'.
    global_open("/dev", None).unwrap().mkdir("vda2");

    let script = global_open("/", None)
        .unwrap()
        .touch("test_script.sh")
        .unwrap();
    script.writeat(0, include_bytes!("test_script.sh")).unwrap();

    #[cfg(target_arch = "loongarch64")]
    const PLATFORM_STR: &str = "la";
    #[cfg(target_arch = "riscv64")]
    const PLATFORM_STR: &str = "rv";

    run_busybox(
        libc_spec!("/mnt/", "/busybox"),
        &["sh", "/test_script.sh", PLATFORM_STR],
        &[
            "HOME=/root",
            "PATH=/bin:",
            "USER=cirno",
            "LOGNAME=cirno",
            "TERM=xterm-256color",
            "SHELL=/bin/sh",
            "SHLVL=1",
            "LANG=C",
        ],
    );

    fn run_busybox(path: &str, args: &[&str], envp: &[&str]) {
        let memspace = {
            let busybox = filesystem_abstractions::global_open(path, None).unwrap();
            let busybox = busybox.readall().unwrap();

            MemorySpaceBuilder::from_raw(&busybox, path, args, envp).unwrap()
        };

        let task = ProcessControlBlock::new(memspace);
        {
            let mut pcb = task.pcb.lock();
            pcb.cwd = String::from("/mnt");
            pcb.is_initproc
                .store(true, core::sync::atomic::Ordering::Relaxed);
        }
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

        let memspace = {
            let elf = filesystem_abstractions::global_open(path, None)
                .expect("Failed to open path")
                .readall()
                .expect("Failed to read file");

            MemorySpaceBuilder::from_raw(&elf, path, args.unwrap_or(&[]), envp.unwrap_or(&[]))
                .unwrap()
        };

        let task = ProcessControlBlock::new(memspace);

        let directory = path::get_directory_name(path).unwrap();
        task.pcb.lock().cwd = String::from(directory);

        spawn_task(task);
        threading::run_tasks();
    }

    // mount and umount tests requires a node at '/dev/vda2'.
    global_open("/dev", None).unwrap().mkdir("vda2");

    preliminary_test(libc_spec!("/mnt/", "/basic/uname"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/write"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/times"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/brk"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/gettimeofday"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/getpid"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/getppid"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/getcwd"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/sleep"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/fork"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/clone"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/yield"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/exit"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/wait"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/waitpid"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/execve"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/pipe"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/dup"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/dup2"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/openat"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/open"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/close"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/read"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/mount"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/umount"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/mkdir_"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/chdir"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/fstat"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/getdents"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/unlink"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/mmap"), None, None);
    preliminary_test(libc_spec!("/mnt/", "/basic/munmap"), None, None);
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

    debug_info();
    logging::init();
    drivers::initialize();
    kernel::init();

    memory::init();

    extern "C" {
        fn ekernel();
    }

    let machine = drivers::machine();
    let bottom = VirtualAddress::as_physical(ekernel as usize);
    allocation::init(bottom, machine.memory_end());

    // Must be called after allocation::init because it depends on frame allocator
    paging::init(PageTable::borrow_current());

    processor::init_processor_pool();

    BOOTED.store(true, core::sync::atomic::Ordering::Relaxed);

    filesystem_abstractions::initialize();
    ProcDeviceInode::setup();

    let sda = machine.create_block_device_at(0);
    filesystem_abstractions::global_mount_inode(&(sda as Arc<dyn IInode>), "/dev/sda", None)
        .unwrap();

    filesystem::global_mount_device("/dev/sda", "/mnt", None).unwrap();

    let etc = global_open("/etc", None).unwrap();
    let passwd = etc.touch("passwd").unwrap();
    passwd.writeat(0, b"cirno:x:0:0::/root:/bin/bash").unwrap();

    let kmsg = KernelMessageInode::new();
    global_mount_inode(&kmsg, "/dev/kmsg", None).unwrap();
    global_mount_inode(&kmsg, "/proc/kmsg", None).unwrap();

    let rtc_time = current_timespec();

    let seed =
        (((rtc_time.tv_nsec as u64) << 32) | machine.query_performance_frequency()) ^ 0xdeadbeef;

    log::info!("Setting up global rng with seed: {seed}");

    rng::initialize(seed);
}

#[no_mangle]
#[allow(named_asm_labels)]
unsafe extern "C" fn __kernel_start_main() -> ! {
    __kernel_init();

    platform_abstractions::init_trap();

    main();

    platform_abstractions::machine_shutdown(false)
}

fn debug_info() {
    #[cfg_accessible(platform_specific::init_serial)]
    platform_specific::init_serial();

    legacy_println!("Welcome to BAKA OS!");

    platform_abstractions::print_bootloader_info();
}
