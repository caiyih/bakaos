#![no_std]
#![no_main]
#![feature(future_join)]
#![feature(cfg_accessible)]
#![feature(alloc_error_handler)]
#![feature(stmt_expr_attributes)]

use core::{ops::Deref, ptr::addr_of};

use abstractions::IUsizeAlias;
use address::{PhysicalAddress, VirtualAddress, VirtualAddressRange};
use alloc::sync::Arc;
use allocation::FrameAllocator;
use hermit_sync::SpinMutex;
use kernel_abstractions::IKernel;
use linux_syscalls::{ISyscallResult, SyscallContext};
use linux_task::LinuxProcess;
use linux_task_abstractions::ILinuxTask;
use memory_space::MemorySpaceBuilder;
use mmu_abstractions::IMMU;
use mmu_native::PageTable;
use platform_abstractions::{return_to_user, UserInterrupt};
use platform_specific::{legacy_println, virt_to_phys, SyscallPayload};
use task_abstractions::ITask;
use threading::block_on;
use trap_abstractions::ISyscallPayloadMut;

use crate::{
    kernel::Kernel, serial::KernelSerial, syscalls::handle_syscall_async, tty::TeletypewriterFile,
};

extern crate alloc;

mod kernel;
mod logging;
mod serial;
mod syscalls;
mod tty;

// The entry point from the underlying HAL
// We need to do some initialization and then begin our main logic
#[no_mangle]
extern "C" fn __kernel_start_main() -> ! {
    legacy_println!("Hello world from guest kernel!");
    legacy_println!(
        "Platform: {}",
        platform_specific::PLATFORM_STRING.to_str().unwrap()
    );

    logging::init();

    extern "C" {
        fn ekernel(); // the end of the kernel, see linker script
    }

    #[link_section = ".bss.heap"]
    static KERNEL_HEAP_START: [u8; 0] = [0; 0];

    global_heap::init(VirtualAddressRange::from_start_len(
        VirtualAddress::from_ptr(addr_of!(KERNEL_HEAP_START)),
        0x0080_0000, // refer to the linker script
    ));

    let allocator_bottom = virt_to_phys(ekernel as usize);
    let allocator_top = allocator_bottom + 0x400000; // 4 MB

    let allocator = Arc::new(SpinMutex::new(FrameAllocator::new(
        PhysicalAddress::from_usize(allocator_top),
        PhysicalAddress::from_usize(allocator_bottom),
    )));

    let serial = KernelSerial::new();

    let kernel = Kernel::new(serial, allocator);

    match main(kernel) {
        Ok(_) => unsafe { platform_abstractions::machine_shutdown(false) },
        Err(msg) => panic!("{}", msg),
    }
}

fn main(kernel: Arc<Kernel>) -> Result<(), &'static str> {
    let task = create_task(&kernel);
    let ctx = kernel.create_syscall_contenxt_for(task.clone());

    let task_closure = run_task(ctx);

    // activate page tabe for the task
    {
        let mmu = task.process().mmu();

        kernel.activate_mmu(mmu.lock().deref());
    }
    let exit_code = block_on!(task_closure);

    if exit_code != 0 {
        return Err("Task failed");
    }

    Ok(())
}

#[cfg(target_arch = "loongarch64")]
static ELF: &[u8] = include_bytes!("../../hello-world/hello-la");

#[cfg(target_arch = "riscv64")]
static ELF: &[u8] = include_bytes!("../../hello-world/hello-rv");

fn create_task(kernel: &Kernel) -> Arc<dyn ILinuxTask> {
    let mmu: Arc<SpinMutex<dyn IMMU>> =
        Arc::new(SpinMutex::new(PageTable::alloc(kernel.allocator())));

    let builder = MemorySpaceBuilder::from_elf(&ELF, "", &mmu, &kernel.allocator()).unwrap();

    let task = LinuxProcess::new(builder, 0);
    {
        let process = task.process();

        let tty = TeletypewriterFile::new(kernel.serial());

        let mut fd_table = process.fd_table().lock();
        fd_table.allocate_at(tty.clone(), 0).unwrap();
        fd_table.allocate_at(tty.clone(), 1).unwrap();
        fd_table.allocate_at(tty, 2).unwrap();
    }

    task
}

async fn run_task(ctx: SyscallContext) -> i32 {
    let task = &ctx.task;

    while !task.status().is_exited() {
        let reason = return_to_user(task.trap_context_mut());

        if let Some(exit_code) = handle_user_trap(&ctx, reason).await {
            return exit_code as i32;
        }
    }

    0
}

async fn handle_user_trap(sys_ctx: &SyscallContext, return_reason: UserInterrupt) -> Option<usize> {
    match return_reason {
        UserInterrupt::Syscall => {
            let task = &sys_ctx.task;
            let trap_ctx = task.trap_context_mut();
            let mut payload = SyscallPayload::new(trap_ctx, sys_ctx);

            payload.move_to_next_instruction();

            let ret = handle_syscall_async(&payload).await;

            log::info!("[syscall return]: {:?}", ret);

            let ret = ret.as_usize();

            if task.status().is_exited() {
                return Some(ret);
            }

            payload.trap_ctx.set_return_value(ret);
        }
        _ => unimplemented!("Unhandled user interrupt: {:?}", return_reason),
    }

    None
}
