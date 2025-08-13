# 虚拟内存设计

rcore 与 xv6 采用跳板页表的方式实现虚拟内存管理。在这种方式下，内核和用户空间的页表是分开的，通过虚拟地址的最后一页，储存跳板函数来实现内核空间和用户空间的切换。

其本质是为了防止内核和用户空间的地址空间重叠，从而更好地产生一个虚拟的，干净的地址空间。

我们的设想是，既然我们能够把跳板页放在最高处，那我们就能够把整个内核都放在最高处，这样内核一直在高处运行，可以随时访问当前进程的用户空间，既不需要切换页表，也不需要进行分段的内存拷贝。

由于内核还需要控制整个物理内存，因此事实上我们需要将一半的地址空间划分给内核，这样内核才能够分配和管理所有的物理内存。因此我们从第 256 GB 开始，映射物理内存。并且将低 256 GB 的地址空间划分给用户空间。这样，内核空间和用户空间就不会重叠而用户和内核都能够尽可能地使用整个地址空间。

## 虚拟内存布局

```
#[link_section = ".data.prepage"]
static mut PAGE_TABLE: [usize; 512] = {
    let mut arr: [usize; 512] = [0; 512];
    arr[2] = (0x80000 << 10) | 0xcf;
    arr[0x100] = (0x00000 << 10) | 0xcf;
    arr[0x101] = (0x40000 << 10) | 0xcf;
    arr[0x102] = (0x80000 << 10) | 0xcf;
    arr
};
```

*注： 0xcf 是 ADSRWX权限，(XXX << 10) 是为了将页号转化为 PTE 的格式*

在启动时，由于我们必须先写入 satp，再跳转到高地址。在写入 satp 后，PC 仍然处于低地址，因此我们必须持有 0x80200000 附近的一些指令的 RX 权限。

为了映射一页 4K 的内存，我们必须再分配 `2 * 4096` 字节的内存用于二级和一级页表。与其映射这 4K 内存，我们不如直接映射 1 GB 内存，这样我们就无需浪费 8K 的内存。

接着，我们将0x00000000到0xc0000000的三个 giant page 映射到第 256，257, 258 个 giant page。这样，内核仅需使用 `0xffff_ffc0_0000_0000` 这个偏移，就可以访问到物理内存。

在跳转到高地址后，用于启动的 giant page 就不再需要了。因此对于所有的进程的页表，我们只需要添加三个 giant page，即可访问到整个物理内存。

我们可以为高位的内核映射添加 S 权限，这样用户程序就无法访问到内核的代码和数据。我们还可以为 `sstatus` 设置 SUM 位，这样内核就可以直接访问用户空间的内存。

经过这样的虚拟内存设计后，内核就可以合法地直接访问用户空间的有效地址。但是，如果用户传入了一个无效地址，但这个地址又不是 NULL，假如内核去访问这个地址，就会发生 page fault。这不是我们想要的，因此我们需要在内核中加入一些检查，以确保内核不会访问到无效地址。

我们设计了一系列 `PageGuard` 结构，利用 Fluent creation pattern 和 RAII 的特性，来确保内核不会访问到无效地址。这样，内核就可以安全地访问用户空间的内存。

利用 `PageGuard`，内核可以轻松访问用户空间的指针，值，数组，甚至不定长字符串而无需担心安全性问题。

例如，对于 `sys_uname` 系统调用，我们可以使用以下代码来轻松实现。可以随意查看我们的系统调用实现，你会发现这样的代码几乎无处不在，足以见得 `PageGuard` 的强大。

```rust
fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
    let p_utsname = ctx.arg0::<*mut UtsName>();

    match ctx
        .tcb
        .borrow_page_table()
        .guard_ptr(p_utsname)
        .mustbe_user()
        .mustbe_readable()
        .with_write()
    {
        Some(mut guard) => {
            guard.write_to(0, "Linux");
            guard.write_to(1, "BakaOS");
            guard.write_to(2, "9.9.9");
            guard.write_to(3, &format!("#9 {}", constants::BUILD_TIME));
            guard.write_to(4, "RISC-IX");
            guard.write_to(5, "The most intelligent and strongest Cirno");

            Ok(0)
        }
        None => Err(-1),
    }
}
```
