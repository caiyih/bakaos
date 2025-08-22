# Hardware abstraction layer

我们**独立**为内核开发了一个硬件抽象层（Hardware Abstraction Layer），用来抽象硬件的细节。

该抽象层同时支持 `riscv64` 和 `loongarch64`，并且可以很容易地扩展以支持更多平台。它由以下几个 crate 组成：

## `platform-abstractions`

最底层的 crate，提供基础的硬件抽象。该 crate 负责引导启动（boot）和中断（interrupt）处理。引导部分负责启用虚拟内存，设置高半内核空间（higher half kernel space），并完成一些平台相关的初始化操作，然后直接跳转到内核代码。中断部分采用 coroutine（协程）方式处理，这意味着当发生中断时，它会保存当前上下文（context），然后返回到你进入用户态（user space）时的代码。这允许内核以异步（asynchronous）方式调度任务。

## `platform-specific`

提供平台特定功能的 crate，包括平台特定的 syscall id、trap context、串口 IO、访问平台特定寄存器（包括通用寄存器和部分 CSR）、处理器核心 id，以及虚拟地址到物理地址的转换能力。

该库主要用于为 `platform-abstractions` 提供底层平台支持。但也被用于`drivers`，`filesystems-abstractions` 以及 `pagetable` 库。

## `page_table`

平台无关的页表抽象 crate。用于通过分页机制（paging mechanism）管理虚拟内存。该 crate 使用了激进的内联（aggressive inlining）、常量传播（constant propagation）和分支消除（branch elimination），以实现几乎零开销（zero overhead）（启用部分功能时）。

## `drivers`

提供硬件抽象和访问接口的驱动 crate。包含 RTC 访问、性能计数器（performance counter）、块设备（block device）等平台特定硬件接口。

该库的定位不在于掩盖硬件细节，而是根据已有的硬件抽象进行抽象层的实现。
