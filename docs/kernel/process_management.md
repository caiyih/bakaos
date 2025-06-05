## 内核模块详解：进程与线程管理

BakaOS 在设计其进程与线程管理机制时，力求在提供类UNIX功能的同时，利用Rust的异步特性来提升并发处理能力。

### 1. 任务模型 (Task Model)

#### 1.1 设计目标

- **BakaOS 的目标：** 建立一个清晰、灵活的任务模型，能够有效地管理和区分进程级资源（如内存空间、文件描述符表）和线程级资源（如CPU寄存器状态、栈），并为调度和同步提供基础。

#### 1.2 核心思想与设计

BakaOS 采用一种分层的任务抽象：

- **`ProcessControlBlock` (PCB - 进程控制块):**
  
  - 代表一个传统意义上的“进程”所拥有的核心资源和属性。
  
  - **资源隔离单位：** 每个 `ProcessControlBlock` 独享一套 `MemorySpace` (虚拟内存空间)、`FileDescriptorTable` (文件描述符表)、`TaskMemoryMap` (内存映射区域记录)以及当前工作目录 (`cwd`) 等。
  
  - **进程属性：** 包含进程ID (`id`，通常是主线程的TID)、父进程信息 (`parent`)、退出状态 (`exit_code`, `status`)、子进程列表、可执行文件路径 (`executable`)、命令行参数 (`command_line`)、以及进程全局的统计信息 (`stats`) 和 `FutexQueue` (用于futex同步)。
  
  - 一个 `ProcessControlBlock` 可以包含一个或多个执行单元（线程）。

- **`TaskControlBlock` (TCB - 任务控制块):**
  
  - 代表一个可被内核调度的独立执行单元，可以理解为一个“线程”（尽管BakaOS的实现更侧重于用户态任务的协程化管理）。
  
  - **调度单位：** 每个 `TaskControlBlock` 拥有独立的 `TrackedTaskId` (全局唯一的任务/线程ID)、`TaskTrapContext` (保存用户态陷入内核时的寄存器状态)、任务状态 (`task_status`，如Ready, Running, Exited)、用户态和内核态的计时器 (`timer`, `kernel_timer`)、以及用于异步唤醒的 `Waker`。
  
  - **共享与私有：**
    
    - 同一进程内的多个 `TaskControlBlock` 会共享同一个 `Arc<SpinMutex<ProcessControlBlock>>`，这意味着它们共享相同的地址空间、文件描述符等进程级资源。
    
    - 每个 `TaskControlBlock` 拥有自己独立的寄存器上下文和栈。

- **任务ID管理 (`TrackedTaskId`, `TaskIdAllocator`):**
  
  - `TaskIdAllocator` 负责分配和回收全局唯一的任务ID。
  
  - `TrackedTaskId` 使用RAII机制，在其 `Drop` 时自动回收ID，防止ID泄漏。

- **任务状态 (`TaskStatus`):**
  
  - 定义了任务在其生命周期中所处的不同阶段，如 `Uninitialized`、`Ready`、`Running`、`Exited`、`Zombie`。状态变迁由调度器和任务自身（如执行 `exit` 系统调用）驱动。

#### 1.3 实现要点

- **关键类型：**

- **核心操作：**
  
  - `ProcessControlBlock::new()`: 创建一个新的进程。
  
  - `TaskControlBlock::fork_process()`: 创建一个新进程。
  
  - `TaskControlBlock::fork_thread()`: 在同一进程内创建一个新线程（共享大部分资源）。
  
  - `tid::allocate_tid()`: 分配任务ID。

### 2. 调度 (Scheduling)

调度器负责决定在某个CPU核心上，哪个可运行的任务（线程/协程）应该获得执行权。

#### 2.1 设计目标

- **BakaOS 的目标：** 利用Rust的 `async/await` 机制，实现一个轻量级、高效的调度框架，特别适合处理I/O密集型和事件驱动的用户态任务。

#### 2.2 核心思想与设计

BakaOS的调度核心是基于**异步任务（Stackless Coroutines）**：

- **异步任务循环 (`task_loop`):**
  
  - 每个用户态任务（由一个 `TaskControlBlock` 代表）的执行逻辑被封装在一个 `async fn task_loop(tcb: Arc<TaskControlBlock>)` 函数中。
  
  - 这个异步函数构成了任务的整个生命周期：从初始化、进入用户态执行、处理中断/系统调用、再返回用户态，直到任务退出。
  
  - 当任务需要等待外部事件（如I/O完成、定时器到期、Futex唤醒）时，它会在处理系统调用或中断的异步逻辑中 `.await` 某个Future。这会导致 `task_loop` 对应的Future返回 `Poll::Pending`，从而让出CPU执行权。

- **基于 `async-task` 和 `futures` 的调度器 (`threading` crate):**
  
  - BakaOS使用 `async-task` crate 来将 `Future` 包装成可运行的 `Runnable` 对象。
  
  - `Scheduler` (位于 `crates/threading/src/executor.rs`) 维护一个或多个队列（ `VecDeque<Runnable>`）来存放准备就绪的 `Runnable` 任务。
  
  - **`spawn(task_future)`:** 当一个新的异步任务（如 `task_loop`）被创建时，它会被包装并提交给 `Scheduler`。
  
  - **`run_tasks()`:** 这是调度器的核心循环。它会从队列中取出 `Runnable` 任务并执行其 `.run()` 方法。`.run()` 会驱动 `Future` 的 `poll` 方法。
    
    - 如果 `poll` 返回 `Poll::Ready`，表示该 `Future`（即 `task_loop`）已完成（任务退出）。
    
    - 如果 `poll` 返回 `Poll::Pending`，表示任务正在等待某个事件。此时，`Runnable` 任务通常会通过其 `Waker` 确保在事件就绪时能被重新调度（即再次放入 `Scheduler` 的队列中）。
  
  - **`yield_now()`:** 提供了一个简单的异步原语，允许任务主动让出CPU，其实现是让当前Future返回 `Poll::Pending` 并立即重新调度自己。

- **协作式与潜在的抢占点 (Cooperative and Potential Preemption):**
  
  - 当前主要描述的是一个**协作式多任务**模型：任务只有在执行 `.await` 并返回 `Poll::Pending` 时才会主动让出CPU。
  
  - **抢占点：** 真正的抢占通常由外部中断（如时钟中断）触发。当中断发生并返回到内核时，内核可以选择不立即恢复当前被中断的任务，而是调用 `run_tasks()` 来执行其他就绪任务，从而实现抢占。`return_to_user` 之后的 `user_trap_handler_async(...).await` 是一个关键点，如果这里面有 `yield_now()` 或者等待其他事件，就自然地融入了协作式调度。

#### 2.3 实现要点

- **关键函数/结构：**
  
  - `task_loop(tcb: Arc<TaskControlBlock>)` (位于 `kernel/src/scheduling.rs`): 每个用户任务的异步执行体。
  
  - `Scheduler` (位于 `crates/threading/src/executor.rs`): 存储和分发 `Runnable` 任务。
  
  - `threading::spawn()`: 将 `Future` 转换为 `Runnable` 并加入调度队列。
  
  - `threading::run_tasks()`: 主调度循环。
  
  - `threading::yield_now()`: 主动让出CPU的异步原语。
  
  - `Waker`: 由 `async-task` 或内核的事件通知机制（如 `FutexQueue`）创建和使用，用于唤醒等待的任务。

- **与中断处理的结合：** `user_trap_handler_async` 在处理完中断或系统调用后，如果任务未退出，会继续 `task_loop` 的下一次迭代。如果中断处理本身是异步的（例如等待I/O），则 `task_loop` 会自然地 `.await`。

#### 2.4 优势

- **高效处理I/O密集型任务：** `async/await` 天然适合非阻塞I/O，当任务等待I/O时，CPU可以去执行其他任务，提高系统吞吐量。

- **简化并发逻辑：** 相比传统基于线程和锁的并发模型，`async/await` 在很多情况下可以写出更易于理解和维护的并发代码（尽管内核中锁仍然是必要的）。

- **轻量级上下文：** Stackless coroutine的切换开销通常比有栈线程的上下文切换要小。

### 3. 上下文切换 (Context Switching)

#### 3.1 设计目标

- **BakaOS 的目标：** 实现一个高效且正确的上下文切换机制，支持用户态任务与内核态执行流之间的平滑过渡。

#### 3.2 核心思想与设计

BakaOS的上下文切换主要发生在用户态任务陷入内核（Trap）以及从内核返回用户态时：

- **陷入内核 (Trap to Kernel):**
  
  - 当用户态任务执行了系统调用、发生缺页异常、或被外部中断（如时钟中断）打断时，CPU会自动切换到内核态，并跳转到预设的陷阱处理入口（由 `stvec` 或 `loogarch`的 `EENTRY` 等寄存器指定）。
  
  - **硬件辅助保存部分状态：** CPU硬件通常会自动保存一些关键寄存器，如用户态的PC (保存到 `sepc` 或 `ERA`)、及陷入原因 (`scause` 或 `ESTAT`)等。
  
  - **软件保存完整上下文 (`__on_user_trap`):**   陷阱处理入口处的底层汇编代码（如 `platform-abstractions/src/<arch>/trap/user.rs` 中的 `__on_user_trap`）负责保存用户态任务的**所有通用寄存器**以及其他必要的CPU状态（如 `sstatus` 或 `PRMD`）到一个预定义的结构中，即 `TaskTrapContext`。

- **从内核返回用户态 (`return_to_user`):**
  
  - 当内核处理完系统调用或中断，并决定返回到（可能是同一个或另一个）用户态任务时，会调用 `platform_abstractions::return_to_user(&mut TaskTrapContext)` 函数。
  
  - **软件恢复上下文 (`__return_to_user`):**  该函数内部的底层汇编代码（ `platform-abstractions/src/<arch>/trap/user.rs` 中的 `__return_to_user`）负责从传入的 `TaskTrapContext` 中恢复用户态任务的CPU状态。

- **浮点寄存器处理 (`FloatRegisterContext`):**
  
  - 为了优化性能，浮点寄存器的保存和恢复通常是**惰性**的。
  
  - CPU状态寄存器（如RISC-V的 `sstatus.FS` 位）会标记浮点单元的状态（Off, Initial, Clean, Dirty）。
  
  - 只有当浮点单元被实际使用过（状态变为Dirty）并且发生上下文切换时，才需要保存浮点寄存器。恢复时也类似。
  
  - `TaskTrapContext` 中包含一个 `FloatRegisterContext` 结构来存储浮点寄存器。`TaskControlBlock::fregs` 字段提供了对此的访问，并有 `activate_restore()` 和 `snapshot()` 等方法。

#### 2.3 实现要点

- **关键结构/函数：**
  
  - `TaskTrapContext` (定义于 `platform-specific/src/<arch>/context.rs`): 保存用户态陷入时的完整CPU状态。
  
  - `__on_user_trap` (位于 `platform-abstractions/src/<arch>/trap/user.rs`): 用户态陷入内核的底层汇编入口，负责保存上下文。
  
  - `__return_to_user` (位于 `platform-abstractions/src/<arch>/trap/user.rs`): 从内核返回用户态的底层汇编函数，负责恢复上下文。
  
  - `platform_abstractions::return_to_user()`: Rust层调用，触发向用户态的转换。
  
  - `FloatRegisterContext` (定义于 `platform-specific/src/<arch>/context.rs`): 管理浮点寄存器的保存与恢复。

- **CPU控制寄存器：**
  
  - RISC-V: `stvec` (陷阱向量基址), `sscratch` (内核栈/上下文指针暂存), `sepc` (异常PC), `scause` (异常原因), `sstatus` (状态寄存器)。
  
  - LoongArch: `EENTRY` (异常入口), `CSR.KSAVE0-3` (用于保存临时寄存器), `CSR.ERA` (异常PC), `CSR.ESTAT` (异常状态), `CSR.PRMD` (先前模式信息)。

#### 2.4 优势

- **浮点寄存器惰性保存减少了上下文切换的开销**

- **平台抽象：** `platform-abstractions` 试图将大部分上下文切换逻辑封装起来，提供统一的 `return_to_user` 接口。

### 4. 进程创建与销毁 (Process Creation and Termination)

#### 4.1 设计目标

- **BakaOS 的目标：** 实现类UNIX的进程创建（`fork`, `clone`, `execve`）和终止（`exit`, `wait`）原语。

#### 4.2 核心思想与设计

BakaOS通过系统调用来实现进程的生命周期管理：

- **`fork()` / `clone()` (`CloneSyscall`):**
  
  - **目的：** 创建一个新的执行流。
  
  - **`fork()` ：** 创建一个几乎与父进程完全相同的子进程副本。子进程拥有父进程数据段、堆和栈的副本。文件描述符表通常也会被复制，但文件表项（描述打开文件的状态，如文件指针）可能是共享的或复制的，取决于具体实现和`O_CLOEXEC`等标志。
  
  - **`clone()` ：** `clone` 是更底层的创建原语，允许调用者精确控制子任务与父任务共享哪些资源（如虚拟内存空间、文件描述符表等）。这是通过 `TaskCloneFlags` 参数来实现的。
    
    - 如果设置了 `TaskCloneFlags::VM`，则子任务与父任务共享同一个 `MemorySpace`（即创建线程）。
    
    - 否则，会为子任务创建一个新的 `MemorySpace`。
  
  - **实现流程：**
    
    1. 分配一个新的 `TaskControlBlock` (和 `ProcessControlBlock`，如果不是创建线程)。
    
    2. 复制或共享父任务的资源（内存空间、文件描述符表等）到新的TCB/PCB，具体行为由 `TaskCloneFlags` 控制。
    
    3. 复制父任务的 `TaskTrapContext` 到子任务，这样子任务从 `return_to_user` 返回时，看起来就像是从 `fork`/`clone` 系统调用返回一样。
    
    4. 对父任务，`fork`/`clone` 返回子任务的ID。对子任务，返回0。
    
    5. 将子任务加入调度队列。

- **`execve()` (`ExecveSyscall`):**
  
  - **目的：** 用一个新的程序镜像替换当前进程的内存空间、代码、数据和栈。进程ID保持不变。
  
  - **实现流程：**
    
    1. 解析新的ELF可执行文件（通过 `MemorySpaceBuilder::from_raw()`）。
    
    2. 创建一个全新的 `MemorySpace`，包含新程序的代码段、数据段、BSS段，并设置新的用户栈和堆区。
    
    3. 更新当前任务的 `TaskTrapContext`，使其 `sepc` (或 `ERA`) 指向新程序的入口点 (`entry_pc`)，栈指针指向新栈的栈顶 (`stack_top`)，并设置好传递给新程序的 `argc`, `argv`, `envp`。
    
    4. 释放旧 `MemorySpace` 所占用的资源（页帧等，通过 `Drop` 实现）。
    
    5. 当前任务被重新放入调度队列（或直接返回用户态执行新程序）。

- **`exit()` / `exit_group()` (`ExitSyscall`, `ExitGroupSyscall`):**
  
  - **目的：** 终止当前任务或整个进程（线程组）。
  
  - **实现流程：**
    
    1. 设置任务的 `task_status` 为 `TaskStatus::Exited`。
    
    2. 保存退出码 (`exit_code`)。
    
    3. 释放任务占用的资源（如关闭文件描述符，释放内存映射区域，最终由 `TaskControlBlock` 和 `ProcessControlBlock` 的 `Drop` 实现来完成大部分页帧和ID的回收）。
    
    4. 如果这是进程中的最后一个线程（或者调用了 `exit_group`），则整个进程被标记为退出。
    
    5. 通知父进程（如果父进程正在 `wait`）。任务状态变为 `TaskStatus::Zombie` 直到父进程回收其状态。

- **`wait()` / `waitpid()` (`sys_wait4_async`):**
  
  - **目的：** 父进程等待其子进程状态改变（通常是终止），并获取子进程的退出状态。
  
  - **实现流程 (异步)：**
    
    1. 检查指定ID的子进程（或任一子进程，如果 `pid == -1`）的状态。
    
    2. 如果子进程已经退出 (`TaskStatus::Exited` 或 `TaskStatus::Zombie`)：
       
       - 获取其退出码。
       
       - 如果用户提供了状态指针 (`p_code`)，则将退出状态写入用户空间。
       
       - 从父进程的子进程列表中移除该子进程。
       
       - 返回子进程的ID。
    
    3. 如果子进程尚未退出：
       
       - 如果指定了 `WNOHANG` (通过 `nohang` 参数)，则立即返回0或错误。
       
       - 否则，父进程（的当前 `task_loop`）会 `.await` 一个与子进程状态变化相关的事件（例如，通过 `Futex` 或其他同步原语，或者简单地 `yield_now()` 并依赖调度器轮询），直到子进程退出或收到信号。

#### 4.3 实现要点 (Implementation Highlights)

- **关键系统调用处理函数：**
  
  - `CloneSyscall`, `ExecveSyscall`, `ExitSyscall`, `ExitGroupSyscall` (位于 `kernel/src/syscalls/task.rs`)。
  
  - `sys_wait4_async` (位于 `kernel/src/syscalls/task_async.rs`)。

- **核心TCB/PCB操作：**
  
  - `TaskControlBlock::fork_process()`, `TaskControlBlock::fork_thread()` 。
  
  - `TaskControlBlock::execve()。
  
  - `MemorySpaceBuilder::from_raw()` (位于 `crates/paging/src/memory.rs`，用于加载ELF并构建内存空间)。
  
  - `TaskStatus` 的状态转换逻辑。
  
  - `FileDescriptorTable::clone_for()`, `FileDescriptorTable::clear_exec()`。

- **标志位：** `TaskCloneFlags` (定义于 `crates/tasks/src/structs.rs`) 控制 `clone` 的行为。
