# Documentation

## Repository 相关

- [项目结构](repository/structure.md)
- [持续集成/部署](repository/continuous-integration.md)

## 内核原理及代码相关

- [内核设计](kernel/README.md)
- [异步调度](kernel/coroutine-scheduling.md)
- [虚拟内存](kernel/virtual-memory.md)
- [内核栈展开](kernel/stack-unwinding.md)
- [Syscall Dispatcher](kernel/syscall-dispatcher.md)
- [文件系统](kernel/filesystem.md)

## 构建小型内核

为了展示内核组件库的高复用性，我们编写了一个小型内核的例子。该内核可以运行这样一个最简单的 Hello world 程序，支持异步系统调用和异步调度，并且无需修改任何代码就能支持 RISC-V 平台和 LoongArch 平台。

- [小型内核](kernel/minimal-kernel.md)
- [小型内核-进阶](kernel/minimal-kernel-advance.md)
