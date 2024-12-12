# 基于协程的调度

该内核采用无栈协程与有栈协程混合的方式进行调度。无栈协程是指协程的栈空间是由调度器动态分配的，而有栈协程是指协程的栈空间是由用户自己分配的。

## 有栈协程

有栈协程模型用于处理中断（包括异常）。由于中断发生的时机不可预知，也并非像无栈协程一样在固定时刻触发，因此我们必须在 trap 发生时保存当前协程的上下文，以便在下次调度时恢复。

中断的发生时机有两种情况：

- 内核态中断
- 用户态中断

对于这两种情况，我们都**必须**保存当前协程的完整上下文，也就是至少所有通用寄存器的值，以便恢复。对于用户态中断，由于我们需要回到内核代码，为了避免安全性问题，我们需要隔离用户程序的栈和内核的栈，因此我们需要保存用户程序的栈指针。对于内核态中断，由于我们使用无栈协程调度，因此我们只需要使用 1 个内核栈，因此我们不需要保存内核栈指针，而可以直接进行内核态中断处理。

然而对于内核态回到用户态的情况，就不一样了。内核态回到用户态的时机是完全固定的，并且使用函数，因此遵循函数调用约定。既然遵循调用约定，在保存内核上下文的时候，我们就可以仅保存 caller-saved 寄存器。这大大减少了上下文的大小。

我们还需要处理浮点寄存器，不过由于浮点寄存器不容易被污染，我们可以在上下问保存完毕后再通过 Rust 代码，根据 sstatus 的 FS 来懒式保存/恢复浮点寄存器。具体查看 `kernel/src/trap/user.rs` 中的 `return_to_user` 函数和`crates/tasks/src/user_task.rs` 中的上下文结构体。

## 无栈协程

无栈协程模型用于调度用户态任务。利用 async/await 机制，我们可以使用 Rust 的异步编程模型来处理多任务。async/await 的函数本质上是一个状态机，这个状态机就是一种无栈协程。

async/await 的核心在于 await。我们知道，协程中的 yield 是将当前协程挂起，而 await 是将当前 async 函数挂起。在 Rust 中，await 可以展开成一个循环和 yield 的组合，这样就可以实现异步编程。例如，对于 `let result = await FooAsync()`，可以展开成：

```rust
let result;
loop {
    let fooFuture = FooAsync(); // FooAsync() 事实上不包含业务逻辑，仅返回一个 Future
    result = match fooFuture.poll() {
        Poll::Ready(val) => break val,
        Poll::Pending => yield,
    }
}
```

这样，我们就可以在一个循环中不断调用 Future 的 poll 方法，直到 Future 返回 Ready。这就是 async/await 的本质。

利用 asnyc/await，我们可以实现一个无栈协程调度器，并且，对于每一个用户态任务，我们可以用一个 async 方法控制其整个生命周期的循环。就像下面一样：

```rust
async fn task_loop(tcb: Arc<TaskControlBlock>) {
    debug_assert!(
        tcb.is_ready(),
        "task must be ready to run, but got {:?}",
        tcb.task_status
    );

    *tcb.task_status.lock() = TaskStatus::Running;

    // 用户程序从此处开始
    while !tcb.is_exited() {
        return_to_user(&tcb);

        // Returned from user program. Entering trap handler.
        // We've actually saved the trap context before returned from `return_to_user`.

        user_trap_handler_async(&tcb).await;
    }

    // Task exited. Do some cleanup like dangling child tasks, etc.
}
```

*注：详细代码可以查看`kernel/src/scheduling.rs`*

只需要实现一个无栈协程调度器，其中每一个协程都是一个`task_loop`状态机，就可以实现对用户进程的异步调度。

本项目的无栈协程调度器位于`crates/threading/executor.rs`。
