# 解析 Rust OS 开发：异步内核、用户态管理与系统调用

在[前文](minimal-kernel.md)中，我们已经成功实现了一个 **基于 BakaOS 的最小化异步内核**，它能够运行用户态的 `Hello, world` 程序，并处理基本的 `syscall` 请求。接下来，我们将深入讲解 **Rust OS 开发的核心概念**，帮助你理解**异步内核、用户态管理、系统调用**的底层原理，并提供更多的代码示例。  

## 1. 异步内核：为什么要异步？

### 1.1 传统内核 vs. 异步内核

在传统的 OS 设计中，**内核是同步的**，即：
- 每个用户进程被调度时，占据 CPU 直到完成或被中断
- 内核代码一般是**阻塞式**的，例如：读取文件、等待网络数据、执行 `syscall`
- 需要复杂的 **线程管理**（如 Linux 内核的 `wait`、`epoll`、中断驱动等）

而 **异步内核** 采用 **非阻塞** 的执行方式：
- 进程遇到 I/O 或 `syscall` 时，不会阻塞整个线程
- 内核调度器可以继续执行其他任务
- 通过 `async/await` 机制，实现更高效的事件驱动处理  

### 1.2 Rust 中的异步支持

Rust 提供了 **`async` 和 `await`** 语法，但在 OS 内核开发中，我们无法直接使用标准库的 `tokio` 等运行时，因此需要 **手写最小化的异步调度器**。  

我们之前的 `run_task_async()` 方法本质上就是 **异步执行用户任务的最小实现**：

```rust
async fn run_task_async(mem_space: MemorySpace, mut trap_ctx: TaskTrapContext) -> i32 {
    let mut exit_code: Option<u8> = None;

    while exit_code.is_none() {
        unsafe { mem_space.page_table().activate() }; // 激活用户进程的页表
        let interrupt_type = platform_abstractions::return_to_user(&mut trap_ctx);

        match interrupt_type {
            UserInterrupt::Syscall => {
                let mut syscall_ctx = SyscallContext::new(
                    &mut trap_ctx,
                    SyscallPayload { pt: mem_space.page_table() },
                );

                exit_code = handle_syscall_async(&mut syscall_ctx).await;
            }
            _ => unimplemented!("Unsupported interrupt type"),
        }
    }
    exit_code.unwrap() as i32
}
```

> **核心逻辑**
> - **切换用户空间**（`return_to_user`）
> - **等待 `syscall` 或中断**
> - **用 `async` 方式处理系统调用**
> - **非阻塞地继续执行任务**

### 1.3 调度器

一个 `run_task_async()` 异步函数用于控制一个程序的执行，但是一个真正的内核并不是只能运行一个程序的。

我们的例子中，`block_on!()`宏就是一个最小的调度器。让我们来看看它的实现：

```rust
let mut future = unsafe { Pin::new_unchecked(future) };
let waker = Waker::noop();
let mut context = Context::from_waker(waker);

loop {
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => return value,
        Poll::Pending => continue,
    }
}
```

我们不断地轮询`future`，直到它返回一个值。每次轮询，事实上就是在执行这个 Future 的代码，所以当它返回一个值时，我们就可以认为这个 Future 已经完成，即用户程序已经完成。

目前它非常简单，只是不断地轮询一个任务。但是你一定可以猜到，如果我们能够同时轮询多个 Future，我们就可以**同时运行多个用户程序**。

这其实非常简单！我们可以用一个队列来储存所有的 Future，每次从队列中取出一个，然后执行它。当一个任务结束时，我们将它从队列中移除，并继续执行。直到队列为空时，我们的就调度地运行了所有的用户程序。

它看起来就像下面这样：

```rust
fn run_many_tasks(tasks: &mut VecDeque<Box<dyn Future<Output = i32>>>) {
    let mut future = unsafe { Pin::new_unchecked(future) };
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    while !tasks.is_empty() {
        let mut future = tasks.pop_front().unwrap();

        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => (), // 报告一个特定的任务完成，但是不能返回，因为可能还有其他任务
            Poll::Pending => tasks.push_back(future), // 将任务放回队列，后续继续执行
        }
    }
}
```

我们还可以动态地向调度器中添加任务，只要你能够访问到 tasks 队列。

这就是最简单地 Round Robin 调度算法。要更改调度策略，你只需要将`VecDeque` 和 `pop_front()` 更换为你自定义的容器和决策函数。

### **1.4 任务切换与调度优化**  

上面的 `run_many_tasks()` 方法实现了 **最基础的轮询调度**（Round Robin），但它仍然存在几个问题：  

1. **任务切换没有抢占机制**  
   - 目前的调度方式是 **协作式（Cooperative）** 的，也就是 **只有当任务主动让出 CPU（返回 `Poll::Pending`）时，调度器才会运行其他任务**。  
   - 如果某个任务 **一直占用 CPU，不返回 `Pending`**，那么其他任务就无法执行，整个系统会卡死。  

2. **缺少定时调度（Preemptive Scheduling）**  
   - 真实的操作系统通常使用 **时钟中断（Timer Interrupt）**，定期 **强制打断** 任务，让出 CPU，保证每个任务都有机会运行。  
   - 这样即使某个任务长时间运行，操作系统也能定期切换到其他任务。  

---

### **1.5 任务切换：加入时钟中断**  

我们可以使用 **定时器中断** 来触发任务切换，让我们的调度器更加高效。  

在 **LoongArch64 或 RISC-V64 架构** 上，我们通常这样做：  

1. **初始化时钟定时器**（如 `sbi_set_timer()` 或 `timer_set_next_event()`）  
2. **在时钟中断发生时，保存当前任务的上下文**  
3. **选择下一个任务，恢复其上下文**  
4. **返回用户态，执行新的任务**  

但是这事实上并不需要调度器的参与，我们只需要再调用 `return_to_user` 前，设置一个时钟中断，然后 CPU 就会在特定的时间触发它，使得内核重新回到 `run_task_async`，使得我们有能力切换任务。

```diff
async fn run_task_async(mem_space: MemorySpace, mut trap_ctx: TaskTrapContext) -> i32 {
    let mut exit_code: Option<u8> = None;

    while exit_code.is_none() {
        unsafe { mem_space.page_table().activate() }; // Activating the page table should be a consideration.

+        set_next_timer(10); // Triggers a timer interrupt after 10ms

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
```

这样，每次 **定时器中断** 发生时，我们都会能够切换任务，让多个任务能够并行执行。我们还可以测量或计算每一个任务的时间片，从而更加精细地控制每一个任务的调度。

## **2. 用户态管理：内核如何加载和运行用户程序？**  

### **2.1 什么是用户态？**  

在现代操作系统中，CPU 运行在 **两种模式**：
- **内核态（Kernel Mode）**：可以访问所有硬件资源
- **用户态（User Mode）**：受限访问，只能执行用户程序  

在 LoongArch64 和 RISC-V64 架构中，CPU **启动时运行在内核态**，我们需要手动创建 **用户态环境**，然后让用户程序运行在其中。

### **2.2 Rust OS 如何创建用户空间？**

在我们的内核中，`MemorySpaceBuilder::from_raw()` 负责 **加载用户程序** 并 **创建虚拟地址空间**：

```rust
fn create_user_space(program: &[u8]) -> MemorySpaceBuilder {
    MemorySpaceBuilder::from_raw(program, "", &[], &[]).unwrap()
}
```

- **程序二进制** (`program: &[u8]`) 直接嵌入到内核
- **MemorySpaceBuilder** 解析 ELF 格式，创建用户态的**虚拟地址空间**

然后，我们需要 **创建用户进程的上下文**（`TaskTrapContext`）：

```rust
fn create_task_context(mem_space: &MemorySpaceBuilder) -> TaskTrapContext {
    TaskTrapContext::new(
        mem_space.entry_pc.as_usize(), // 用户程序的入口地址
        mem_space.stack_top.as_usize(), // 栈顶地址
        mem_space.argc,  // 参数数量
        mem_space.argv_base.as_usize(),
        mem_space.envp_base.as_usize(),
    )
}
```

> **总结**：
> - `MemorySpaceBuilder` 加载 ELF 二进制，根据 ELF 文件构建用户空间，并构建页表
> - `TaskTrapContext` 记录用户程序的**上下文（寄存器状态）**，**入口地址**等信息
> - `return_to_user()` 将 CPU 切换到用户态

## **3. 系统调用（Syscall）：内核与用户程序的交互**  

### **3.1 什么是系统调用？**  

**用户态程序不能直接访问内核资源（比如 I/O、文件系统、网络）**，必须通过 `syscall` 进入内核，让内核帮忙执行任务。

在 LoongArch64，`syscall` 是通过 `syscall 0x0` 指令触发的：

```assembly
li.d    $a7, 64  # write 系统调用号
li.d    $a0, 1   # 文件描述符 stdout
la.abs  $a1, message
la.abs  $a2, message_end
sub.d   $a2, $a2, $a1  # 计算 message 长度
syscall 0x0  # 触发系统调用
```

> - `a7` 存放 **系统调用号**（如 `64` 表示 `write`）
> - `a0, a1, a2...` 传递 **参数**
> - 触发 `syscall 0x0` 进入内核

### **3.2 Rust OS 如何处理 syscall？**  

在 Rust 内核中，当 `syscall` 等中断发生时，内核会执行一小段代码，然后从 `return_to_user()` 函数中返回到 `run_task_async()` 中，达到*拦截* `syscall` 的效果。然后内核调用 `handle_syscall_async()` 处理各种 `syscall`：

```rust
async fn handle_syscall_async(ctx: &mut SyscallContext<'_>) -> Option<u8> {
    const SYS_WRITE: usize = 64;
    const SYS_EXIT: usize = 93;

    ctx.move_to_next_instruction(); // PC still points to syscall instruction, we want to skip it

    let return_value = match ctx.syscall_id() {
        SYS_WRITE => {
            let (fd, p_buf, len) = (
                ctx.arg0::<isize>(),
                ctx.arg1::<VirtualAddress>(),
                ctx.arg2::<usize>(),
            );

            assert_eq!(fd, 1, "Only stdout is supported");

            match ctx.pt.guard_slice(p_buf.as_ptr::<u8>(), len).mustbe_user().with_read() {
                Some(guard) => {
                    legacy_print!("{}", core::str::from_utf8(&guard).unwrap());
                    guard.len() as isize
                }
                None => -14, // Bad address
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

> **核心逻辑**
> - **解析 `syscall_id`**：判断是 `write` 还是 `exit`
> - **读取参数**：用户程序传递的 `fd`、`buf`、`len`
> - **安全访问用户内存**：防止越界访问
> - **执行操作**：
>   - `write` → 输出字符串
>   - `exit` → 终止进程

## **总结**
- Rust OS 采用 **异步内核**，通过 `async/await` 处理任务
- **用户态管理** 需要创建并激活页表，然后切换到用户模式
- **系统调用（Syscall）** 是用户程序访问内核资源的唯一途径
