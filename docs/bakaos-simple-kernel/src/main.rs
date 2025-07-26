// SPDX-License-Identifier: MPL-2.0
// This demo project is adapted from https://asterinas.github.io/book/ostd/a-100-line-kernel.html

#![no_std]
#![no_main]
#![feature(future_join)]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::{ptr::addr_of, usize};

use abstractions::IUsizeAlias;
use address::{VirtualAddress, VirtualAddressRange};
use paging::{
    page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder, MemorySpace,
    MemorySpaceBuilder, PageTable,
};
use platform_abstractions::UserInterrupt;
use platform_specific::{
    legacy_print, legacy_println, virt_to_phys, ISyscallContext, ISyscallContextMut, ITaskContext,
    TaskTrapContext,
};
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

    #[link_section = ".bss.heap"]
    static KERNEL_HEAP_START: [u8; 0] = [0; 0];

    global_heap::init(VirtualAddressRange::from_start_len(
        VirtualAddress::from_ptr(addr_of!(KERNEL_HEAP_START)),
        0x0080_0000, // refer to the linker script
    ));

    let allocator_bottom = virt_to_phys(ekernel as usize);
    let allocator_top = allocator_bottom + 0x400000; // 4 MB

    allocation::init(allocator_bottom, allocator_top);
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
    let trap_ctx = create_task_context(&mem_space);
    let mem_space = mem_space.memory_space;

    let exit_code = block_on!(run_task_async(mem_space, trap_ctx)); // Run the async task
                                                                    // You can also write `run_task_async(task).await;` if you are in an async context

    match exit_code {
        0 => Ok(()),
        _ => Err("User task exited with non-zero exit code"),
    }
}

fn create_user_space(program: &[u8]) -> MemorySpaceBuilder {
    MemorySpaceBuilder::from_raw(program, "", &[], &[]).unwrap()
}

fn create_task_context(mem_space: &MemorySpaceBuilder) -> TaskTrapContext {
    TaskTrapContext::new(
        mem_space.entry_pc.as_usize(),
        mem_space.stack_top.as_usize(),
        mem_space.argc,
        mem_space.argv_base.as_usize(),
        mem_space.envp_base.as_usize(),
    )
}

type SyscallContext<'a> = platform_specific::SyscallContext<'a, SyscallPayload<'a>>;

// You can add more fields that you need for handling syscalls.
// You may heard something called task control block (TCB).
// And the [`SyscallPayload`] is actually the minimal version of TCB.
struct SyscallPayload<'a> {
    pub(crate) pt: &'a PageTable,
}

// This async function controls the execution of the user task
// And returns its exit code
async fn run_task_async(mem_space: MemorySpace, mut trap_ctx: TaskTrapContext) -> i32 {
    let mut exit_code: Option<u8> = None;

    while exit_code.is_none() {
        unsafe { mem_space.page_table().activate() }; // Activating the page table should be a consideration.

        // This method call returns when a trap occurs
        let interrupt_type = platform_abstractions::return_to_user(&mut trap_ctx);

        match interrupt_type {
            UserInterrupt::Syscall => {
                let mut syscall_ctx = SyscallContext::new(
                    &mut trap_ctx,
                    SyscallPayload {
                        pt: mem_space.page_table(),
                    },
                );

                // See it? You can handle syscalls in an async context
                exit_code = handle_syscall_async(&mut syscall_ctx).await;
            }
            _ => unimplemented!("Unsupported interrupt type"),
        }
    }

    exit_code.unwrap() as i32
}

// Returns non to continue execution,
// or Some(exit code) to terminate the task with a given exit code
async fn handle_syscall_async(ctx: &mut SyscallContext<'_>) -> Option<u8> {
    use platform_specific::syscall_ids::{SYSCALL_ID_EXIT, SYSCALL_ID_WRITE};

    ctx.move_to_next_instruction(); // skip the instruction that triggers the syscall

    let return_value = match ctx.syscall_id() {
        SYSCALL_ID_WRITE => {
            let (fd, p_buf, len) = (
                ctx.arg0::<isize>(),
                ctx.arg1::<VirtualAddress>(),
                ctx.arg2::<usize>(),
            );

            assert_eq!(fd, 1, "Only stdout is supported");

            match ctx
                .pt
                .guard_slice(p_buf.as_ptr::<u8>(), len)
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
        SYSCALL_ID_EXIT => {
            let exit_code = ctx.arg0::<u8>();

            return Some(exit_code);
        }
        _ => unimplemented!(),
    };

    ctx.set_return_value(return_value as usize);

    None
}
