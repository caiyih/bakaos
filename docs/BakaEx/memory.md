# BakaEx 的 MMU 抽象

传统的内核直接使用 Page table 来管理内存。它的语义被限定在仅是管理内存的映射上。BakaEx 内核不使用 Page table 这一术语，相似的概念被抽象成 MMU，即 Memory Management Unit。MMU 原本是 CPU 的一个内部模块，它不仅负责着虚拟地址到物理地址的翻译，同时还处理着翻译后对内存的访问。

具体来说，我们以 LoongArch 的一条访存指令为例：

```text
ld.d rd, rj, si12
```

这条指令将从 rj + si12 的 _地址_ 处加载一个 doubleword，并保存在 rd 寄存器中，是一条非常经典的访存指令。

根据 LoongArch ISA 的定义，这条指令的语义为：

```text
LD.D:
    vaddr = GR[rj] + SignExtend(si12, GRLEN)    # 计算要访问的 Virtual Address
    AddressComplianceCheck(vaddr)               # 检查地址对齐
    paddr = AddressTranslation(vaddr)           # 查找页表项、检查映射权限、并翻译成一条物理地址
    GR[rd] = MemoryLoad(paddr, DOUBLEWORD)      # 从物理地址处加载数据
```

在我们的 MMU 抽象（以下称为 IMMU）中，访问用户内存时，以上行为将由 IMMU 检查。让我们先来看一下 IMMU 的定义。

IMMU 是一个接口，用来管理虚拟内存映射，以及用户内存的访问，具体来说，它的接口定义如下：

```rust
pub trait IMMU {
    fn map_single(
        &mut self,
        vaddr: VirtualAddress,
        target: PhysicalAddress,
        size: PageSize,
        flags: GenericMappingFlags,
    ) -> PagingResult<()>;

    fn remap_single(
        &mut self,
        vaddr: VirtualAddress,
        new_target: PhysicalAddress,
        flags: GenericMappingFlags,
    ) -> PagingResult<PageSize>;

    fn unmap_single(&mut self, vaddr: VirtualAddress) -> PagingResult<(PhysicalAddress, PageSize)>;

    fn query_virtual(
        &self,
        vaddr: VirtualAddress,
    ) -> PagingResult<(PhysicalAddress, GenericMappingFlags, PageSize)>;

    fn create_or_update_single(
        &mut self,
        vaddr: VirtualAddress,
        size: PageSize,
        paddr: Option<PhysicalAddress>,
        flags: Option<GenericMappingFlags>,
    ) -> PagingResult<()>;

    #[doc(hidden)]
    fn inspect_framed_internal(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        callback: &mut dyn FnMut(&[u8], usize) -> bool,
    ) -> Result<(), MMUError>;

    #[doc(hidden)]
    fn inspect_framed_mut_internal(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        callback: &mut dyn FnMut(&mut [u8], usize) -> bool,
    ) -> Result<(), MMUError>;

    fn translate_phys(
        &self,
        paddr: PhysicalAddress,
        len: usize,
    ) -> Result<&'static mut [u8], MMUError>;

    fn read_bytes(&self, vaddr: VirtualAddress, buf: &mut [u8]) -> Result<(), MMUError>;

    fn write_bytes(&self, vaddr: VirtualAddress, buf: &[u8]) -> Result<(), MMUError>;

    fn platform_payload(&self) -> usize;

    fn map_buffer(&self, vaddr: VirtualAddress, len: usize) -> Result<&[u8]>, MMUError>;

    fn map_buffer_mut(&self, vaddr: VirtualAddress, len: usize) -> Result<&mut [u8]>, MMUError>;

    fn unmap_buffer(&self, vaddr: VirtualAddress);

    #[doc(hidden)]
    #[cfg(not(target_os = "none"))]
    fn register_internal(&mut self, vaddr: VirtualAddress, len: usize, mutable: bool);

    #[doc(hidden)]
    #[cfg(not(target_os = "none"))]
    fn unregister_internal(&mut self, vaddr: VirtualAddress);
}
```

它包含以下功能：

- `map_single`：给定用户空间的虚拟地址、权限、页大小、目标物理地址，将虚拟地址映射到目标物理地址。
- `remap_single`：给定用户空间的虚拟地址、新的目标物理地址、权限，将虚拟地址重新映射到新的目标物理地址。
- `unmap_single`：给定用户空间的虚拟地址，取消虚拟地址的映射。
- `query_virtual`：给定用户空间的虚拟地址，返回虚拟地址所映射的物理地址、权限、页大小。
- `create_or_update_single`：给定用户空间的虚拟地址、页大小、目标物理地址、权限，创建或更新虚拟地址所映射的物理地址、权限。
- `platform_payload`：返回一个平台相关的数据，用于在 MMU 中保存一些平台相关的数据，在裸机程序中，它是根页表的物理地址，用于传递给内核激活。

以上的功能本质属于传统的 Page table 的职责，因此返回的错误类型均为 PagingError，与后面的 MMUError 不同。

- `inspect_framed_internal`：给定用户空间的虚拟地址、长度、回调。使用帧翻译算法，按帧访问虚拟地址，并调用回调函数处理。
- `inspect_framed_mut_internal`：给定用户空间的虚拟地址、长度、回调。使用帧翻译算法，按帧访问虚拟地址，并调用回调函数处理。与上面一个方法的唯一区别在于它还要求内存具有可写权限。
- `translate_phys`：获得一个物理地址在线性映射空间可访问的虚拟地址，例如，高半内核常将整个物理内存映射到线性空间，这样，整个物理内存都可以通过这个线性空间进行访问。
- `read_bytes`：给定虚拟地址、缓冲区，将虚拟地址处的数据复制到缓冲区中。
- `write_bytes`：给定虚拟地址、缓冲区，将缓冲区中的数据复制到虚拟地址处。

剩余的 4 个方法我会在后文中专门介绍。

除此之外，IMMU 还有一些扩展方法：

```rust
impl dyn IMMU {
    pub fn inspect_framed(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        mut callback: impl FnMut(&[u8], usize) -> bool,
    ) -> Result<(), MMUError>;

    pub fn inspect_framed_mut(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        mut callback: impl FnMut(&mut [u8], usize) -> bool,
    ) -> Result<(), MMUError>;

    pub fn import<T: Copy>(&self, vaddr: VirtualAddress) -> Result<T, MMUError>;
    pub fn export<T: Copy>(&self, vaddr: VirtualAddress, value: T) -> Result<(), MMUError>;

    #[cfg(not(target_os = "none"))]
    pub fn register<T>(&mut self, val: &T, mutable: bool) -> VirtualAddress;
    #[cfg(not(target_os = "none"))]
    pub fn unregister<T>(&mut self, val: &T);
}
```

`inspect_framed` 和 `inspect_framed_mut` 方法是对 `inspect_framed_internal` 和 `inspect_framed_mut_internal` 方法的封装，它们分别用于检查帧中的数据。

`import` 方法用于从指定的用户空间的虚拟地址直接读取一个值并返回，与之对应的 `export` 方法则用于将一个值写入指定的用户空间的虚拟地址。

## MMU 职责的体现

前往提到，MMU 的职责不仅完成地址翻译，还需要：检查对齐、检查权限、检查物理帧（如果映射指向无效的物理地址，仍然不可访问），然后才能允许访问。

我们的 `inspect_framed_internal`、`inspect_framed_mut_internal`、 `read_bytes`、 `write_bytes` 方法在内部都会进行这些检查。

以 `read_bytes` 为例，其内部实现如下：

- 检查地址是否满足给定类型的对齐要求，不满足直接报错
- 调用 `inspect_framed_internal`，一边检查页帧，一边进行内存拷贝。具体来说，对于每一个页帧:
- 检查相应的映射是否存在
- 检查映射权限是否满足要求： User + Read
- 检查被映射的帧是否存在，这一步仅在测试环境中存在
- 进行处理内存拷贝

可以看到，IMMU 在访问内存前，就和 MMU 做了一样的事情，不仅确保了整个用户空间的访问过程是安全的，还将整个访问过程抽象起来。

为什么我们可以这样做？当我们尝试 inspect 一段内存的时候，就可以认为我们要访问一段内存。既然将来一定要访问，那我们就可以提前检查这段内存。这一过程与 Rust 的借用规则相似，即持有一个可变引用就等价于你要修改值。如果持有可变引用而不修改值的话，持有是没有意义的。同理，inspect 一段内存却不访问，那 inspect 就不必要存在。

## TestMMU 的实现

为了在宿主主机中模拟用户空间，并且能够隔离、访问用户空间，我们引入了 TestMMU。它是对 IMMU 机制的一个实现，只是仅用于宿主主机中运行。

```rust
pub struct TestMMU {
    alloc: Arc<SpinMutex<dyn ITestFrameAllocator>>,
    mappings: Vec<MappingRecord>,
}

struct MappingRecord {
    phys: PhysicalAddress,
    virt: VirtualAddress,
    flags: GenericMappingFlags,
    len: usize,
    from_test_env: bool,
}
```

TestMMU 的具体实现位于 test-utilities 中。它的实现非常简单，它维护着一个向量，而向量的元素就是一个一个从虚拟地址到物理地址的映射记录。

每个记录包含：

- `phys`: 虚拟地址
- `virt`: 目标物理地址
- `flags`: 映射的页大小
- `len`: 映射的页数量
- `from_test_env`: 是否来自测试环境

我们同样以 `read_bytes` 为例，TestMMU 将做以下事情：

按帧粒度将物理地址映射到虚拟地址。每次翻译时遍历向量，找到相应的映射，确认权限，并向内存分配器 alloc 检查对应得物理地址是否有效。

这个过程确保了裸机实现与测试环境中的语义一致性。

值得一提的是，所谓的物理地址，在宿主主机运行时，其实是测试程序的虚拟地址。这也是我们引入`from_test_env`字段的原因。

我们显然不希望测试时所有的内存都需要在用户空间中分配，这样太麻烦了。观察一下我们对 sys_write 系统调用的测试代码：

```rust
#[test]
fn test_received_from_testenv() {
    let (kernel, alloc, mmu) = setup_kernel_with_memory();

    let test_file = TestFile::new();
    let mut fd_table = FileDescriptorTable::new();
    fd_table.allocate(test_file.clone());

    let (_, task) = TestProcess::new()
        .with_memory_space(Some(MemorySpace::new(mmu.clone(), alloc)))
        .with_fd_table(Some(fd_table))
        .build();

    let ctx = SyscallContext::new(task, kernel);

    let buf = b"Hello, world";
    mmu.lock().register(buf, false); // let the mmu know about the buffer

    let ret = block_on!(ctx.sys_write(0, buf.into(), buf.len()));

    assert_eq!(ret, Ok(buf.len() as isize));

    assert_eq!(test_file.content(), buf);
}
```

显然我们想能够直接将测试环境的内存传入到系统调用的实现中进行测试，这样会大大提高测试效率。因此我们引入了 `register` 和 `unregister` 方法，用于将内存区域注册到 MMU 中，并解除注册。注册完成后，当通过 IMMU 访问用户内存时，就会确认这段内存是有效的，并且能够进行访问。

## `map_buffer` 与 `unmap_buffer`

这两个方法的理念借鉴了 Vulkan 的 vkMapMemory 和 Direct3D 的 Map 机制。Vulkan 和 Direct3D 都是图形 API，用于创建 3D 图形程序。

在这两个库中，这个机制被用于从 CPU 内存中上传内存到 GPU 显存。具体来说，当调用这个函数后，驱动程序会将如 VertexBuffer 的数据映射到特定的内存区域，然后图形程序可以将顶点数据复制到这个区域。调用 Unmap 后，顶点数据被上传到 GPU 显存（实际是提交命令队列时上传，这里简化了过程便于理解）。

IMMU 的 `map_buffer` 与 `unmap_buffer` 方法与这个过程类似，只不过它是 （被测内核）CPU 到 （测试框架）CPU 的。调用这个方法后，特定的区域的内存被连续映射到 CPU 的内存空间。当然，`map_buffer` 要先确保映射有效才会返回。返回后，内核可以访问返回的连续内存，这个过程避免了帧翻译的开销。`unmap_buffer` 提交后，该映射不再有效。

这一机制存在的原因是为了对高半地址空间访问用户内存做优化。在高半地址空间中，整个地址空间被分为两个区域。内核运行在一个相当高的地址，而用户程序运行在一个相对较低的地址，确保了它们的地址空间不会相互冲突。

这样做的好处是，内核可以访问用户空间中的任何地址，只要它们映射有效，内核访问就不会引发异常。

在 TestMMU 的实现中，我们不能像高半内核空间一样直接访问连续的用户空间。并且由于我们模拟的用户空间尽管在虚拟地址中是连续的，我们对内存的直接访问是通过测试框架的虚拟内存访问的（可以理解成 Mocked 内核的物理帧），实际并不一定连续。因此我们需要一个显式的 map 动作，来准备一片连续的区域。并在 unmap 的时候同步更改到实际的用户空间。

尽管在 TestMMU 中会比较麻烦，但是该机制在裸机实现中没有引入任何开销。我们可以在该过程中对映射进行权限检查，这些检查本身也是必要的。检查完成后，会直接通过传入的 vaddr 构建内存切片。高半内核设计允许内核直接访问用户空间的内存，而不需要进行帧翻译。
