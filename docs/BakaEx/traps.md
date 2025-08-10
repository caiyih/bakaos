# 当心语义陷阱

在我们的研究中，我们发现个别系统调用的语义极具迷惑性，甚至许多精通内核的开发人员也会误解它们。尽管从传统内核的视角使用错误的语义看待它们并没有什么问题。但是如果你第一次从 BakaEx 的视角看看待这些系统调用，你的错误认知会阻碍你正确理解系统调用的语义。

本文中，我们将尝试探讨这些系统调用的准确语义，避免陷入语义陷阱。

## 一、问题概述

在 BakaEx 的设计中，**系统调用只负责改变进程/任务的状态或元数据**（例如：建立新的内存空间、标记退出、创建子任务），而 **是否“立即执行／调度／退出资源回收”完全由调度器决定**。许多开发者（包括经验丰富者）仍然沿用“调用 = 执行 / 调度 / 结束”的直觉：这是传统整机视角下的隐含假设，但在 BakaEx 的可测试架构中这个假设会导致严重的认知偏差与错误测试。

对三个典型系统调用的误解汇总：

- `sys_exit`：往往被期待“直接让进程停止执行并释放资源”。但在内核的代码实现中（不只是 BakaEx）中它只**标记状态并存储退出码**，真正从运行队列移除、资源回收、wait/wakeup 时机仍由调度器/回收策略决定。
- `sys_fork`：常被期待“立刻运行子进程 / 切换到子进程”，但内核通常只是**创建子 TCB 并把它放入调度队列**——是否运行、何时运行取决于调度器策略。
- `sys_execve`：常被期待“立刻开始运行新 ELF”，但实际行为是**替换进程的 address space / 程序镜像**；新镜像何时开始执行（什么时候把 CPU 分配给该线程）同样取决于调度器。

因此，单元测试不可基于“系统调用同时含有 scheduling side-effect”的期望；而要基于“系统调用做了哪些被保证的状态改变”来编写断言。

BakaEx 设计中，调度器（Scheduler）是一个完全可控的测试驱动组件。
系统调用本身只修改内核状态（如任务控制块 TCB、调度队列、内存空间等），至于何时运行、是否运行，完全由调度器决定。

这种设计导致从传统内核视角直接迁移预期，会引发严重的认知偏差。本说明旨在澄清这一点，并提供正确的单元测试策略。

## 二、根源（为什么在 BakaEx 中问题更明显）

1. **依赖注入 + 测试时手工控制 TCB**
   单测通常在 host 上构造 `SyscallContext` 并手动驱动 TCB / TrapContext。这暴露了系统调用与调度器之间的分界：系统调用的“标记”并不会自动触发宿主线程切换，测试代码可以在退出后继续操作这个 task 的 TCB，从而打破传统的“进程一旦 exit 就不再可用”的直觉。

2. **把 scheduler 的决策留给外部（好处 + 代价）**
   这种分离使得测试更灵活，但也让“谁负责”的界限不清——开发者不总是意识到调度器才是决定进程是否被运行/移除的唯一实体。

3. **历史语义混淆**
   传统文档 / 教科书常把 `fork/exec/exit` 的效果以“外显”方式描述（比如“fork 创建并返回两次”），但未明确哪个行为是“状态改变”哪个是“调度后果”。在实机上，两者经常合并观察到，导致误解被放大。

## 三、推荐的“思维模型” —— 把语义分成两层

在 BakaEx 中，把每次系统调用分成 **两类语义/效果** 并在文档中强制区分：

1. **Guaranteed effects（保证的状态改变）**
   这些是系统调用必须完成并在测试中可断言的效果（例如：`task.exit_code = 42`、`child exists with pid X`、`task.memory_space replaced with new_image`）。

2. **Scheduler effects（调度器相关的可选/不可保证后果）**
   包括“马上切换到子进程”、“把进程从 runqueue 中移除并回收资源”、“让 exec 后的程序立即执行”等。这些**不由 syscall 保证**，而是调度器/回收器策略的结果。

在文档、代码注释和测试用例中都应明确标出每个 syscall 的两层语义。

## 四、对各个 syscall 的具体说明、易错点与测试建议

### 1) `sys_exit`

**保证的效果（可断言）**

- 将任务状态标为 `Exited`（或类似状态）。
- 存储退出码（exit_code）。
- 如果需要，向等待的父进程/等待队列发送“可收获”信号（如果实现了该逻辑，称为 “notify”）。

**不可保证的效果（不可断言，除非测试环境注入特定 scheduler）**

- 任务立刻被移出 runqueue。
- 立即释放所有资源（某些资源可能在 reaper/GC 中释放）。
- 其它任务马上观察到 wait() 的返回。

**测试建议**

- 编写断言只针对 guarantee 部分：`assert!(task.is_exited()); assert_eq!(task.exit_code(), Some(code));`
- 若想测试“后续资源回收”，在测试中注入可控的 scheduler/reaper，让其运行并断言资源被回收。
- 提供两类测试：一类只验证 syscall 的行为；另一类（integration-style）用一个 deterministic scheduler 验证调度/回收行为。

**示例测试片段（伪 Rust）**

```rust
#[test]
fn sys_exit_marks_task_exited() {
    let (ctx, _scheduler) = setup_context_with_noop_scheduler();
    ctx.sys_exit(42).unwrap();

    let t = ctx.task();
    assert!(t.is_exited());
    assert_eq!(t.exit_code(), Some(42));
    // 不断言 runqueue 是否被移除——那是 scheduler 的职责
}
```

### 2) `sys_fork`

**保证的效果**

- 创建一个新的 TCB（child），复制/克隆必要的内核元数据（如 file descriptors、memory space 的 copy-on-write 映射或完整复制，依实现而定）。
- 返回子进程的 PID 给 parent（parent 返回 child_pid），在 child 的寄存器集里设置返回值为 0（这在“实际运行 child 时”才能观察到）。

**不可保证的效果**

- 子进程马上运行并在 CPU 上返回 0（除非 test 中驱动 scheduler）。
- 子进程何时被执行（包括是否永远不执行）。

**常见误解**

- 在单元测试中直接断言 “fork 之后 child 已经执行并返回 0” 是错误的；你只能断言 child 的 TCB 存在并已被加入 runqueue（若 syscall 实现中包含 enqueue），或者仅断言 child 被创建，但 enqueue 仍是实现细节/调度策略。

**测试建议**

- 验证 child 的 TCB 存在、父子资源复制/共享策略正确（例如 file descriptor 表、memory mappings）。
- 验证 child 是否被放入 runqueue（如果你的 syscall 实现做了这步），但把“是否被 scheduler 选择运行”留给 integration 测试。
- 提供 `schedule_once()` 类型的测试 helper：在 test 中注入 deterministic scheduler，手动触发一次调度以模拟 child 执行，然后断言 child 在运行后具有期望的寄存器/行为。

**示例**

```rust
#[test]
fn sys_fork_creates_child_but_not_run() {
    let (ctx, scheduler) = setup_test_context_with_noop_scheduler();
    let child_pid = ctx.sys_fork().unwrap();

    assert!(ctx.kernel().task_exists(child_pid));
    // 如果 syscall enqueues child：
    assert!(scheduler.runqueue_contains(child_pid));
    // 但是无法断言 child 已经执行
}
```

### 3) `sys_execve`

**保证的效果**

- 替换当前任务的 `memory_space`（新的 program image、堆栈、入口点等）。
- 通常会重设某些执行上下文（例如指令指针、栈指针、auxv），但这些在“真正开始执行”之前只是元数据变更。

**不可保证的效果**

- 新 ELF 立刻运行（CPU 上开始执行该 ELF）——这需要 scheduler 把 CPU 分配给该任务并恢复上下文。

**测试建议**

- 断言进程的 memory space 已被替换（可检查 page table /映射/ELF 元信息等）。
- 检查 file descriptor 是否按规范保留或关闭（例如 FD_CLOEXEC）。
- 要验证“新的代码被执行并产生效果”，在测试中需要驱动 scheduler（integration test 或在模拟环境下执行 trap/return）。

**示例**

```rust
#[test]
fn sys_execve_replaces_memory_space() {
    let ctx = setup_syscall_context();
    ctx.sys_execve("/bin/foo", &args, &env).unwrap();

    let mem = ctx.task().memory_space();
    assert!(mem.contains_elf("/bin/foo"));
    // 不断言程序的入口被执行
}
```

## 五、 单元测试的正确观念

在 BakaEx 中，系统调用与调度器的执行时机解耦。
单元测试的目标是验证系统调用的**直接副作用**（数据结构变更、状态标记、资源分配等），而非其在传统内核中可能引发的**即时调度行为**。

测试应遵循以下原则：

1. **断言可观测状态变化**：TCB 内容、调度队列、内存映射等。
2. **避免调度假设**：不验证调用后是否“立即运行”或“立即退出”。
3. **直接验证调度器副作用**：如有必要，可显式检查调度队列的变化。
4. **确保测试可重复、可控**：不依赖实际硬件执行或宿主系统的进程管理。

## 六、 为什么这样设计

- **测试环境限制**：
  在宿主主机中，无法直接从测试框架的异步协程环境中 fork 出真实进程并运行新的 ELF。
- **调度器可控性**：
  将运行时机完全交由调度器，确保测试可以完全复现和控制。
- **消除隐式假设**：
  防止将真实内核的运行时行为误投射到 BakaEx 的测试环境，避免无关失败。

## 七、 总结

BakaEx 的 `sys_fork`、`sys_execve`、`sys_exit` 系统调用在语义上与传统内核有显著差异：

- 它们不会隐含触发即时调度或终止行为。
- 单测应只关注调用本身的直接数据结构变更。
- 对调度结果的任何期望，都必须通过显式调度器调用来实现。

这种方式确保测试结果稳定、可控，并避免因错误预期导致的测试误判。
