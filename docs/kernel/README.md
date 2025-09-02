# 内核设计

内核的入口位于 `kernel/src/main.rs` 中，第一行指令位于 `_start` 符号。启动时被加载到物理地址 `0x80200000` 处，然后开始运行。

该函数仅做临时使用，并不用于进行内核初始化。

```Rust
unsafe extern "C" fn _start() -> ! {
    asm!(
        // Read the hart id
        "mv tp, a0",
        // Read the device tree address
        "mv gp, a1",
        // Setup virtual memory
        // See comments below for details
        "la t0, {page_table}",
        "srli t0, t0, 12", // get the physical page number of PageTabe
        "li t1, 8 << 60",
        "or t0, t0, t1", // ppn | 8 << 60
        "csrw satp, t0",
        "sfence.vma",
        // jump to virtualized entry
        "li t1, {virt_addr_offset}",
        "la t0, {entry}",
        "or t0, t0, t1",
        // Do not save the return address to ra
        "jr t0",
        page_table = sym PAGE_TABLE,
        virt_addr_offset = const constants::VIRT_ADDR_OFFSET,
        entry = sym _start_virtualized,
        options(noreturn)
    )
}
```

该函数首先读取 hart id 和设备树地址，然后设置虚拟内存，然后立即跳转到位于虚拟地址高位的 `_start_virtualized` 函数。从该处开始，内核将在虚拟地址空间中运行。

对于内核*虚拟内存*的设计，请参阅 [虚拟内存](virtual-memory.md)。

在 `_start_virtualized` 函数中，我们首先设置栈指针等与虚拟内存相关的寄存器，并清除 fp, ra，防止回到 `_start_virtualized` 函数甚至之前的位置。然后跳转到 `__kernel_start_main` 真正开始 Rust 代码的执行。

`__kernel_start_main` 中，会进行内核初始化操作。初始化操作分为两种，一种是只需要在启动核心 hart 0 上执行的，另一种是每一个 hart 启动时都要执行的。在第一部分初始化完成后，其他 CPU 核心就可以被 hart 0 唤醒了。

由于初始化操作在未来可能会有所变化，所以这里不再详细描述，请自行查看`__kernel_init`函数，其中的注释函数名已经非常清晰地描述了初始化的过程。

初始化完成后，便到 `main` 函数，内核可以选择串行运行特定测试样例，也可以添加`initproc`并运行到所有进程结束。

当内核正常从 `main` 函数返回时，会调用 SBI 进行关机操作，如果是 QEMU，不会产生非 0 的退出码。如果在运行过程中发生了 panic，内核会进行栈回溯并输出错误信息，然后调用 SBI 进行关机操作，此时 QEMU 会产生非 0 的退出码。

内核输出栈回溯后，栈展开脚本会自动运行，并输出源码级别的栈回溯信息，详细信息请参阅 [栈展开](stack-unwinding.md)。
