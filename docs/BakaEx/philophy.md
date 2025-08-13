# 内核设计哲学

要理解我们如何做到宿主主机态的测试，需要先了解 BakaEx 内核的设计哲学。

如何看待一个内核？在我看来，内核与一个 Web 后端服务器其实一样：

## 1. Request-block-response 模型

在 Web 服务中，一个 _用户_ 通过 _Endpoint_ 向服务端发出 _request_，_用户_ 此时被迫等待，直到服务端返回 response。中间服务端可能暂时挂起该请求，处理其他请求，然后再回来继续处理 User 的请求。

对于一个内核，以上过程完全相似，User program 通过 irq 发起 system call request（其他类型的中断亦是如此），此时 User program 被挂起，内核处理 request，处理结束后返回 response，此时 User program 被唤醒，继续执行。中途内核可能由于各种原因暂停处理该程序的请求（schedule，io yield，sleep ...），但稍后，会恢复处理该程序。

这是几乎是 User program 和内核交互的唯一方式，和一个 Web 服务完全相同。

## 2. 通过依赖注入来组合服务（依赖）

在 Web 服务中，我们常常依赖注入的方式来组合服务。我们有 singleton 生命周期，标识在整个服务运行的生命周期中，仅创建 1 次的服务。也有 scoped（prototype）生命周期，标识在单个个作用域内，仅创建 1 次的服务。在服务运行的生命周期内，我们可以创建多个作用域，对于 Web 服务，场景的做法是，整个服务运行的生命周期作为 singleton，而每个请求的处理过程，作为一个作用域。

### 1. 内核单例服务

内核启动时，创建一个 IFileSystem 一个，它是在内核运行生命周期内的一个 "singleton" 实例，用来管理一整个根文件系统，上面可以挂载各种 ILogicFilesystem。除此之外，还有串口设备，调度器，时钟，内存分配器（帧分配器）等服务，它们最终被组合在 IKernel 中。

我们通过依赖注入的方式来管理内核的依赖，从而实现内核的解耦和可测试性。

不幸的是，几乎所有已有的内核都是将这些依赖作为全局静态变量，这会导致大量问题，是内核在架构层面上不能够被单元测试的**根本原因**：

#### 1. 可测试性差

全局变量使得单元测试变得非常困难：

- 无法轻松地为不同的测试用例模拟不同的依赖行为
- 测试之间会相互影响，因为它们共享全局状态
- 难以进行隔离测试，因为全局状态会在测试间保持

相比之下，依赖注入允许我们在测试时轻松注入模拟对象（mocks）或存根（stubs），使得每个测试都可以在受控环境中独立运行。

#### 2. 紧耦合

使用全局变量会导致代码模块之间紧密耦合：

- 模块直接依赖于具体的全局变量实现
- 难以替换或修改底层实现
- 组件复用性差

依赖注入通过接口抽象解耦组件，使代码更加模块化和灵活。

#### 3. 隐藏的依赖关系

全局变量隐藏了模块之间的真实依赖关系：

- 函数或类的依赖关系不明确，需要查看实现代码才能了解
- 增加了理解和维护代码的难度
- 容易在不明确影响范围的情况下修改全局状态

依赖注入使依赖关系显式化，任何依赖都在构造函数或方法参数中清晰可见。

#### 4. 生命周期管理困难

全局变量的生命周期通常与应用程序相同：

- 难以控制对象的创建和销毁时机
- 可能导致资源泄漏
- 难以实现不同的作用域管理

依赖注入容器可以管理不同生命周期的依赖项，如单例（singleton）、瞬态（transient）和作用域（scoped）等。

#### 5. 并行化问题

在多线程或并发环境中，全局变量会带来额外的复杂性：

- 需要额外的同步机制来保护全局状态
- 容易出现竞态条件
- 难以进行并行测试

依赖注入天然支持并行测试，因为每个测试可以拥有自己独立的依赖实例。

### 2. 请求范围服务

当 _中断请求_ 在一个核心上发生时，内核的硬件抽象层将当前上下文保存在进程调度协程中，并回到该协程中，尝试处理 _中断请求_

对应的 irq handler 会通过 IKernel 对象获取、组合自己需要的 scoped 服务。

我们以一个 syscall handler 为例：

我们可以在栈上获取到当前程序的 ITaskControlBlock 对象，TrapContext 引用等。

加上来自于 IKernel 的 IFileSystem（用于访问文件系统），ITaskManager（可能需要获取其他任务，也可能需要 spawn 任务）、串口设备，Hypervisor Accessor 等一切处理系统调用可能需要的服务，组合成一个 scoped 服务：SyscallContext。

然后将该 SyscallContext 服务 传递给 SyscallDispatcher 对象处理，返回。处理完一些事情后，返回到用户态，当前请求处理完毕。

你可以在我们的 syscalls 库中看到我们是怎么做的：

```rust
pub struct SyscallContext {
    #[allow(unused)]
    pub task: Arc<dyn ITask>,
    #[allow(unused)]
    pub kernel: Arc<dyn IKernel>,
}

impl SyscallContext {
    pub fn new(task: Arc<dyn ITask>, kernel: Arc<dyn IKernel>) -> SyscallContext {
        Self { task, kernel }
    }
}
```

我们有一个 SyscallContext，所有的系统调用都在这个类型上实现。我们在这个对象上调用方法就可以访问系统调用。例如，下面是来自我们的 sys_mmap 系统调用的的一个测试函数。

```rust
#[test]
fn test_syscall_anonymous_mapping_exists() {
    let ctx = setup_syscall_context();

    let ret = ctx.sys_mmap(
        SyscallContext::VMA_BASE,
        4096,
        MemoryMapProt::READ,
        MemoryMapFlags::ANONYMOUS,
        0,
        0,
    );

    let vaddr = VirtualAddress::from_usize(ret.unwrap() as usize);

    let mem = ctx.task.process().memory_space().lock();

    let target_mapping = mem
        .mappings()
        .iter()
        .find(|mapping| mapping.range().start().start_addr() == vaddr);

    assert!(target_mapping.is_some());
}
```

到现在你可能好奇的是为什么 `SyscallContext` 没有包含更多的字段？这样的设计为什么能满足我们的要求？我们将在下一节中详细介绍。
