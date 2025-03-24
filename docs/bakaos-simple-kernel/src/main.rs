// SPDX-License-Identifier: MPL-2.0
// This demo project is adapted from https://asterinas.github.io/book/ostd/a-100-line-kernel.html

#![no_std]
#![no_main]
#![feature(future_join)]
#![feature(alloc_error_handler)]

extern crate alloc;

mod heap_allocator; // provide a heap allocator, you can use slab allocator or buddy system allocator

use alloc::sync::Arc;
use core::usize;

use address::VirtualAddress;
use paging::{
    page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder, MemorySpaceBuilder,
    PageTable,
};
use platform_abstractions::{
    translate_current_trap, ISyscallContext, ISyscallContextBase, SyscallContext, UserInterrupt,
};
use platform_specific::{legacy_print, legacy_println, virt_to_phys, ITaskContext};
use tasks::{ProcessControlBlock, TaskControlBlock, TaskStatus};
use threading::block_on;

// The entry point from the underlying HAL
// We need to do some initialization and then begin our main logic
#[no_mangle]
extern "C" fn __kernel_start_main() -> ! {
    legacy_println!("Hello world from guest kernel!");
    legacy_println!(
        "Platform: {}",
        platform_specific::PLATFORM_STRING.to_str().unwrap()
    );

    extern "C" {
        fn ekernel(); // the end of the kernel, see linker script
    }

    heap_allocator::init();
    allocation::init(virt_to_phys(ekernel as usize), usize::MAX);
    paging::init(PageTable::borrow_current());

    match main() {
        Ok(_) => platform_abstractions::machine_shutdown(false),
        Err(msg) => panic!("{}", msg),
    }
}

fn main() -> Result<(), &'static str> {
    // Compile the hello world program with the command in the document
    let program_binary = include_bytes!("../hello");
    let mem_space = create_user_space(program_binary);
    let task = create_user_task(mem_space);

    let exit_code = block_on!(run_task_async(task)); // Run the async task
                                                     // You can also write `run_task_async(task).await;` if you are in an async context

    match exit_code {
        0 => Ok(()),
        _ => Err("User task exited with non-zero exit code"),
    }
}

fn create_user_space(program: &[u8]) -> MemorySpaceBuilder {
    MemorySpaceBuilder::from_raw(program, "", &[], &[]).unwrap()
}

fn create_user_task(mem_space: MemorySpaceBuilder) -> Arc<TaskControlBlock> {
    ProcessControlBlock::new(mem_space)
}

// This async function controls the execution of the user task
// And returns its exit code
async fn run_task_async(task: Arc<TaskControlBlock>) -> i32 {
    while *task.task_status.lock() < TaskStatus::Exited {
        unsafe { task.borrow_page_table().activate() }; // Activating the page table should be a consideration.

        // This method call returns when a trap occurs
        platform_abstractions::return_to_user(&task);

        match translate_current_trap() {
            UserInterrupt::Syscall => {
                let mut syscall_ctx = SyscallContext::new(task.clone());

                // See it? You can handle syscalls in an async context
                handle_syscall(&mut syscall_ctx).await;
            }
            _ => unimplemented!("Unsupported interrupt type"),
        }
    }

    task.exit_code.load(core::sync::atomic::Ordering::Relaxed)
}

async fn handle_syscall(ctx: &mut SyscallContext) {
    const SYS_WRITE: usize = 64;
    const SYS_EXIT: usize = 93;

    ctx.move_to_next_instruction(); // skip the instruction that triggers the syscall

    let syscall_return = match ctx.syscall_id() {
        SYS_WRITE => {
            let (fd, p_buf, len) = (
                ctx.arg0::<isize>(),
                ctx.arg1::<VirtualAddress>(),
                ctx.arg2::<usize>(),
            );

            assert_eq!(fd, 1, "Only stdout is supported");

            match ctx
                .borrow_page_table()
                .guard_slice(unsafe { p_buf.as_ptr::<u8>() }, len)
                .mustbe_user()
                .with_read()
            {
                Some(guard) => {
                    // guard can be automatically dereferenced as &[u8]
                    legacy_print!("{}", core::str::from_utf8(&guard).unwrap());

                    guard.len() as isize
                }
                None => -14, // bad address
            }
        }
        SYS_EXIT => {
            let exit_code = ctx.arg0::<i32>();

            ctx.exit_code
                .store(exit_code, core::sync::atomic::Ordering::SeqCst);

            *ctx.task_status.lock() = TaskStatus::Exited;

            exit_code as isize
        }
        _ => unimplemented!(),
    };

    ctx.mut_trap_ctx()
        .set_syscall_return_value(syscall_return as usize);
}
