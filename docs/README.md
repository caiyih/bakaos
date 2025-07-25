# Documentation

## 答辩 PPT

- 金山文档在线访问：[https://kdocs.cn/l/ceTW4JnBxtUh](https://kdocs.cn/l/ceTW4JnBxtUh)
- 百度网盘：[https://pan.baidu.com/share/init?surl=yblEKQ_8lBl34ta_t9d8fg&pwd=wtkg](https://pan.baidu.com/share/init?surl=yblEKQ_8lBl34ta_t9d8fg&pwd=wtkg)

## 演示视频

- 百度网盘：[https://pan.baidu.com/s/19RYzUepQxyAHZJ7nXCLJsw?pwd=qemy](https://pan.baidu.com/s/19RYzUepQxyAHZJ7nXCLJsw?pwd=qemy)

## Repository 相关

- [项目结构](repository/structure.md)
- [持续集成/部署](repository/continuous-integration.md)

## 内核原理及代码相关

- [内核架构](kernel/kernel-architecture.md)
- [异步调度](kernel/minimal-kernel-advance.md)
- [虚拟内存](kernel/virtual-memory.md)
- [内核栈展开](kernel/stack-unwinding.md)
- [Syscall Dispatcher](kernel/syscall-dispatcher.md)
- [文件系统](kernel/filesystem.md)
- [进程管理](kernel/process_management.md)
- [协程调度](kernel/coroutine-scheduling.md)
- [内存管理](kernel/memory-overview.md)
- [驱动程序](kernel/drivers.md)
- [硬件抽象层](kernel/hardware-abstraction-layer.md)
- [内核入口](kernel/README.md)

## 构建小型内核

为了展示内核组件库的高复用性，我们编写了一个小型内核的例子。该内核可以运行这样一个最简单的 Hello world 程序，支持异步系统调用和异步调度，并且无需修改任何代码就能支持 RISC-V 平台和 LoongArch 平台。

- [小型内核](kernel/minimal-kernel.md)
- [小型内核-进阶](kernel/minimal-kernel-advance.md)

## Source Generation

本项目随附有一个 Rust Source Generation 框架，旨在为中大型项目提供清晰、高效、可扩展的代码生成支持。它为静态代码生成场景提供统一的抽象与运行时环境，适用于模块化生成任务、符号注册与分析、自动化导出等需求，尤其适合 FFI、DSL 编译器前端、属性宏替代方案等工程场景。

- [项目介绍](source-generation/README.md)
- [安装](source-generation/installation.md)
- [快速开始](source-generation/quickstart.md)
- [架构概览](source-generation/core.md)
- [进阶使用](source-generation/advance.md)
- [基于语法语义分析的代码生成](source-generation/syntax-semantic-analysis.md)
- [注意事项/FAQ](source-generation/notice-faq.md)

