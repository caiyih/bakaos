# 用 100 行 Rust 代码打造跨平台异步内核：利用 BakaOS 代码库

本文将展示一个利于 Baka OS 组件构建的跨平台异步内核，使用大约 100 行 Rust 代码。

我们的小型内核运行这样一个最简单的 Hello world 程序：

```asm
# SPDX-License-Identifier: MPL-2.0
# Minimal hello world program for LoongArch64
# adapted from https://asterinas.github.io/book/ostd/a-100-line-kernel.html

.global _start                      # entry point
.section .text                      # code section
_start:
    li.d    $a7, 64                 # syscall number of write
    li.d    $a0, 1                  # stdout
    la.abs  $a1, message            # address of message         
    la.abs  $a2, message_end
    sub.d   $a2, $a2, $a1           # calculate message len
    syscall 0x0
    li.d    $a7, 93                 # syscall number of exit
    li.d    $a0, 0                  # exit code
    syscall 0x0

.section .rodata                    # read only data section
message:
    .ascii  "Hello, world\n"
message_end:
```

该程序首先加载 `write` 系统调用所需的参数以及系统调用号，然后触发一个系统调用异常，请求操作系统将 `message` 中的内容输出到串口。然后回到用户程序，加载 `exit` 系统调用所需的参数，再次触发一个系统调用异常，请求操作系统退出当前进程。

你可以使用下面的命令编译这个程序：

```bash
loongarch64-linux-gnu-gcc -static -nostdlib hello.S -o hello
```

*注意：你需要安装 `loongarch64-linux-gnu-gcc` 工具链*

你可以使用 `qemu-loongarch64` 运行这个程序：

```bash
qemu-loongarch64-static hello
```

你应该会看到下面的输出：

```bash
$ qemu-loongarch64-static hello
Hello, world
```

目前我们以 LoongArch64 为例，但是我们的内核即使不需要更改代码，也能运行一个类似与上面的面向 RISC-V64 的 Hello world 程序。

我们的内核需要完成以下工作：

1. 创建一个用户空间，将这样一个程序加载到其中。
2. 从内核空间进入到用户空间，并保持它们隔离。
3. 在用户程序返回到内核空间后，对其返回的原因（中断类型）进行识别和相应的处理。
4. 特别是处理 syscall 异常，将系统调用参数传递给用户程序，并返回结果。

我们的内核代码如下，我们添加了一些注释来解释它：

```rust
// SPDX-License-Identifier: MPL-2.0
// This demo project is adapted from https://asterinas.github.io/book/ostd/a-100-line-kernel.html

#![no_std]
#![no_main]
#![feature(future_join)]
#![feature(alloc_error_handler)]

extern crate alloc;

mod heap_allocator; // provide a heap allocator, you can use slab allocator or buddy system allocator

use core::usize;

use abstractions::IUsizeAlias;
use address::VirtualAddress;
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
    const SYS_WRITE: usize = 64;
    const SYS_EXIT: usize = 93;

    ctx.move_to_next_instruction(); // skip the instruction that triggers the syscall

    let return_value = match ctx.syscall_id() {
        SYS_WRITE => {
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
        SYS_EXIT => {
            let exit_code = ctx.arg0::<u8>();

            return Some(exit_code);
        }
        _ => unimplemented!(),
    };

    ctx.set_return_value(return_value as usize);

    None
}
```

上面的代码中，完全没有任何平台特定的代码，因此只要我们的 HAL 提供了一个平台的支持，那么这个内核就能够运行相应架构的 Hello world 程序。

你可以[阅读这里](https://github.com/caiyih/bakaos?tab=readme-ov-file#hardware-abstraction-layer)以了解更多 BakaOS 的硬件抽象层。

并且更为重要的是，我们不需要使用 unsafe 代码，因为 BakaOS 已经提供了一些安全机制来保证安全。在实际开发中，激活页表的工作由 Scheduler 来完成，确保不会出现错误。在这里，尽管它是 unsafe 包裹的，但是我们可以确定它是一个安全的行为。

编译运行上面的内核，你应该会看到下面的输出：

```ascii
Hello world from guest kernel!
Platform: LoongArch64
Hello, world
```

你可以在[这里](https://github.com/caiyih/bakaos/tree/master/docs/bakaos-simple-kernel)找到这个示例的完整代码。
