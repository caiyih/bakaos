# Syscall Dispatcher

`Syscall Dispatcher` 位于 `kernel/src/syscall/mod.rs`。它根据 trap context 中的相应寄存器的值，调用相应的 syscall handler。

本内核使用 async/await 机制实现了异步 IO 机制，但部分 syscall 不需要 IO 操作，而另一部分 syscall 需要 IO 操作。因此，我们将使用两种方式来 dispatch syscall.

## 1. 同步 syscall

对于不需要 IO 操作的 syscall，我们通过 dynamic dispatch，将 syscall handler 对象的 vtable 返回给 syscall dispatcher，然后调用相应的 handler。实现了较好的封装性。

## 2. 异步 syscall

由于异步方法返回的类型是未知的，因此我们无法通过 dynamic dispatch 来调用相应的 handler。因此我们采用 static dispatch 的方式，来静态地调用相应的 handler。

将 syscall handler 分为两类，避免了同步 syscall handler 的不必要状态机分配，减少了内存开销，提高了性能。

