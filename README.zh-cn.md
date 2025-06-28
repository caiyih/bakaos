# Baka OS

| [English](./README.md) | 简体中文 |

_注意：翻译版本的所有内容目前由 ChatGPT 4o 翻译，对于不完整的情况，请参考 [README.md](./README.md)。未来会同步简体中文和 English 的内容。_

<a href="https://github.com/caiyih/bakaos">
    <img alt = "Language Rust" src="https://img.shields.io/badge/language-Rust-orange">
</a>
<a href="https://github.com/caiyih/bakaos">
    <img alt = "Kernel type" src="https://img.shields.io/badge/kernel-UNIX--like-blue">
</a>
<a href="https://github.com/caiyih/bakaos">
    <img alt = "Lines of code" src="https://tokei.rs/b1/github/caiyih/bakaos">
</a>
<a href="https://github.com/caiyih/bakaos/blob/master/LICENSE">
    <img alt = "GitHub license" src="https://img.shields.io/github/license/caiyih/bakaos">
</a>
<a href="https://github.com/caiyih/bakaos">
    <img alt = "GitHub repository size" src="https://img.shields.io/github/repo-size/caiyih/bakaos">
</a>
<a href="https://github.com/caiyih/bakaos/activity">
    <img alt = "GitHub commit frequency" src="https://img.shields.io/github/commit-activity/m/caiyih/bakaos">
</a>
<a href="https://github.com/caiyih/bakaos/activity">
    <img alt="GitHub last commit" src="https://img.shields.io/github/last-commit/caiyih/bakaos">
</a>
<a href="https://github.com/caiyih/bakaos/graphs/contributors">
    <img alt="GitHub contributors" src="https://img.shields.io/github/contributors-anon/caiyih/bakaos">
</a>

![Arch_RV64](https://img.shields.io/badge/Architecture-RISC--V64-green)
![Arch_LA64](https://img.shields.io/badge/Architecture-LoongArch64-red)

<!-- end of line -->

[![Crates Code Quality](https://github.com/caiyih/bakaos/actions/workflows/crates-fmt.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/crates-fmt.yml)
[![Crates Tests](https://github.com/caiyih/bakaos/actions/workflows/crates-tests.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/crates-tests.yml)
[![Kernel Code Quality](https://github.com/caiyih/bakaos/actions/workflows/kernel-fmt.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/kernel-fmt.yml)
[![Kernel CI](https://github.com/caiyih/bakaos/actions/workflows/kernel.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/kernel.yml)
[![Preliminary test](https://github.com/caiyih/bakaos/actions/workflows/preliminary.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/preliminary.yml)

Baka OS 是一个用 Rust 编写的类 UNIX 异步操作系统内核。它面向 RISC-V 和 LoongArch64 架构，开发目标是 2024 年操作系统内核大赛（东北赛区）和 2025 年操作系统内核大赛（国赛）。

借助 Stackless Coroutine 和 CPU Pool，Baka OS 拥有在多核环境下并发运行上千个任务的潜力。

本内核并非基于任何现有项目，而是完全从零开发。凭借多年的 OOP 经验，该内核充分利用了抽象与封装的优势，形成了结构清晰、可复用性强的代码库。

该内核划分为多个 crate，并在宿主机上对各个 crate 进行了测试和检查，保证了代码的高质量、易维护和易调试。我们的设计思想是“逻辑上微内核，物理上宏内核”。结合了**微内核架构设计理念**、**测试驱动开发（TDD）思想**和**宏内核性能优势**的现代操作系统内核设计方案，我们希望达到以下特性：

- **系统性强**：有完整的开发、测试、验证闭环
- **实践导向**：解决了真实开发流程中的效率问题
- **长期扩展性好**：可以无缝接入 formal verification、cross-architecture 测试
- **符合工业级代码质量控制**：Mock + 宿主机测试让内核代码可以做到模块独立、高可测、易回归

未来，我们考虑添加在宿主主机中对裸机的 Mock 实现，以实现对更多的 crates 的宿主主机测试的能力。

微内核通过将代码从内核搬到用户空间，以实现更容易的对内核代码的维护，但是我们通过一种全新的方式，即通过将代码从内核中拆分，尽管仍然在内核空间中运行，但是我们通过宿主主机下的全面测试，保证这些代码的正确性，同样确保了代码的高质量，并且具有宏内核的性能优势。

例如，目前阻碍宿主测试的一个难点是页帧分配，但是我们可以通过宿主主机的堆内存分配，来模拟机器上的页帧分配，以测试这部分依赖的代码，这样，几乎仅仅只有 HAL 层和内核不能够直接在宿主机上进行测试。

## Documentation

_仍在编写中_

详细文档请参考 [`docs`](docs/README.md) 目录（仅提供简体中文版本）。

由于项目仍处于高速开发阶段，文档可能无法及时反映最新改动。如有疑问，请以源码为准。

## Continuous Integration

| Workflow            | Status                                                                                                                                                                   |
| :------------------ | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Crates Code Quality | [![Crates Code Quality](https://github.com/caiyih/bakaos/actions/workflows/crates-fmt.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/crates-fmt.yml) |
| Crates Tests        | [![Crates Tests](https://github.com/caiyih/bakaos/actions/workflows/crates-tests.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/crates-tests.yml)    |
| Kernel Code Quality | [![Kernel Code Quality](https://github.com/caiyih/bakaos/actions/workflows/kernel-fmt.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/kernel-fmt.yml) |
| Kernel Tests        | [![Kernel CI](https://github.com/caiyih/bakaos/actions/workflows/kernel.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/kernel.yml)                   |
| Preliminary Grading | [![Preliminary test](https://github.com/caiyih/bakaos/actions/workflows/preliminary.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/preliminary.yml)  |

本仓库使用持续集成（Continuous Integration）以保证代码质量，防止功能回退。每次推送（push）都会经过 CI 系统的测试与检查，确保代码始终保持高质量和稳定性。

## 开发

开发本项目需要以下工具：

- **Cargo 和 Rust**：本项目使用指定版本的 Rust，具体版本在 `rust-toolchain.toml` 文件中定义。安装 Cargo 后，会自动下载并配置对应版本的 Rust。

然后就没有了，因此你实际上可以直接使用 cargo 来构建本内核。

如果启用了 `lwext4` 功能，还需额外安装以下工具链：

- `loongarch64-linux-musl-cc`：用于为 LoongArch64 架构构建 `lwext4`。
- `riscv64-linux-musl-cc`：用于为 RISC-V64 架构构建 `lwext4`。

如果不需要 `lwext4`，可以直接禁用该功能。目前我们正在考虑迁移至完全由 Rust 实现的 ext4 文件系统，以避免对 C 库的依赖。

## Repo introduction

**重要提示：详细文档请参考 `docs` 目录。**

本仓库主要包含三个部分，分别是 `kernel`、`crates` 和其他子项目。

### `kernel`

`kernel` 目录存放的是操作系统内核的源代码。

该目录下包含一个 `Makefile` 构建系统，输入 `make help` 查看详细用法。

_请注意，这里指的是 `kernel` 目录下的 `Makefile`，而不是仓库根目录的。_

```
BakaOS build system
Usage:
- make help        : 显示此帮助信息
- make build       : 编译操作系统内核
- make run         : 使用 qemu 运行操作系统内核
- make debug       : 启动操作系统内核，并等待 gdb 连接
- make connect     : 启动 gdb 并连接到 qemu
- make clean       : 清理构建产物
- make stacktrace  : 解析并生成 qemu 输出的 stacktrace
- make symboltable : 在 kernel/.disassembled 目录生成当前 ELF 文件的符号表
- make strip       : 精简（strip）内核 ELF 文件，通常无需执行此操作

环境变量：
- ARCH=riscv64(default)|loongarch64            : 选择目标架构
- MODE=debug(default)|release-with-debug       : 选择构建模式
- LOG=TRACE|DEBUG|INFO(default)|WARN|ERROR|OFF : 设置日志等级
```

内核的构建产物位于 `target/<ARCH_TRIPLET>/<MODE>/bakaos`。

上方列出了基本用法，但还有一些注意事项需要说明。

#### Environment variables

环境变量用于控制构建过程。目前支持以下变量：

- `ARCH`：指定目标架构，默认为 `riscv64`，同时支持 `loongarch64`。

- `MODE`：指定构建模式，默认为 `debug`。支持的值包括 `debug`、`release-with-debug` 和 `release`。

- `LOG`：指定日志等级，默认为 `INFO`。更多详情请见下方说明。

#### Publish profile

Cargo 默认支持 `debug` 和 `release` 构建配置，而这些配置也被集成到了本构建系统中。此外，构建系统还提供了一个名为 `release-with-debug` 的额外配置。

`release-with-debug` 用于在 release 级别优化的同时，保留调试符号。这对于调试仅在 release 模式下发生的 bug 十分有用。

当然，下面是你提供的内容的完整翻译和润色版，保持简洁、正式、术语规范，风格与之前保持一致：

#### 调试（Debugging）

要在启用 GDB 的模式下运行内核，只需执行以下命令：

```bash
$ make debug
```

该命令将在构建内核时包含调试符号，并通过 QEMU 启动 GDB 服务器。

连接 GDB 服务器时，可以直接使用 GDB 客户端，也可以运行以下命令简化操作：

```bash
$ make connect
```

此外，项目已支持 Visual Studio Code 的调试功能。打开开发工作区后，直接按下 `F5` 即可启动调试，无需手动执行 `make debug`。

#### 清理（Clean）

要清理内核的构建产物，执行以下命令：

```bash
$ make clean
```

请注意，由于内核与 `crates` 位于不同的工作区，此操作不会清理 `crates` 的构建产物。

#### 栈回溯（Stacktrace）

要解析和生成 QEMU 输出的栈回溯信息，执行以下命令：

```bash
$ make stacktrace
```

通常情况下，无需手动运行该命令。但在仅有输出日志和内核二进制文件的情况下，该命令可用于重现崩溃报告。

CI 工作流会将内核二进制文件和输出日志上传为 CI 产物。当内核在 CI 流水线中发生异常时，可下载相关产物，复现并分析崩溃信息。

#### 日志（Logging）

内核日志基于 `log` crate 实现。可通过 `LOG` 环境变量控制日志级别。例如：

```bash
$ make run LOG=TRACE
```

上述命令将在 `TRACE` 日志级别下运行内核。

需注意，日志级别在编译时已固定。但无须手动处理，`run` 命令会根据指定的日志级别自动重新构建内核。

支持的日志级别共六个，由高到低依次为：

- `ERROR`
- `WARN`
- `INFO`（默认）
- `DEBUG`
- `TRACE`
- `OFF`

`ERROR` 为最高级别，仅输出错误信息；`TRACE` 为最低级别，输出最详细的日志信息。

当日志级别为 `OFF` 时，`log` crate 的日志将完全禁用，但内核仍可能向控制台输出部分提示信息（一般无需关注）。

### `crates`

`crates` 目录包含了内核直接依赖的一些代码。这些代码被实现为独立的 crate，因此即使在宿主机上也可以单独进行测试，而无需在内核中测试。

所有 crate 都注册在一个 cargo workspace 中，所以你只需要将编辑器或语言服务器的工作目录打开到 `crates` 文件夹下，就可以编辑所有 crate。

#### Hardware abstraction layer

我们为内核开发了一个硬件抽象层（Hardware Abstraction Layer），用来抽象硬件的细节。该抽象层同时支持 `riscv64` 和 `loongarch64`，并且可以很容易地扩展以支持更多平台。它由以下几个 crate 组成：

- `platform-abstractions`：最底层的 crate，提供基础的硬件抽象。该 crate 负责引导启动（boot）和中断（interrupt）处理。引导部分负责启用虚拟内存，设置高半内核空间（higher half kernel space），并完成一些平台相关的初始化操作，然后直接跳转到内核代码。中断部分采用 coroutine（协程）方式处理，这意味着当发生中断时，它会保存当前上下文（context），然后返回到你进入用户态（user space）时的代码。这允许内核以异步（asynchronous）方式调度任务。

- `platform-specific`：提供平台特定功能的 crate，包括平台特定的 syscall id、trap context、串口 IO、访问平台特定寄存器（包括通用寄存器和部分 CSR）、处理器核心 id，以及虚拟地址到物理地址的转换能力。

- `drivers`：提供硬件抽象和访问接口的驱动 crate。包含 RTC 访问、性能计数器（performance counter）、块设备（block device）等平台特定硬件接口。

- `page_table`：平台无关的页表抽象 crate。用于通过分页机制（paging mechanism）管理虚拟内存。该 crate 使用了激进的内联（aggressive inlining）、常量传播（constant propagation）和分支消除（branch elimination），以实现几乎零开销（zero overhead）（启用部分功能时）。

### Workspace

强烈推荐使用 Visual Studio Code 来开发此项目。我们提供了一些脚本用于自动生成开发工作区：

- `SetupRV64Workspace.sh`
- `SetupLA64Workspace.sh`

当你运行这些脚本时，它会在你运行脚本的当前目录（cwd）生成一个开发工作区（development workspace）。

本项目通过条件编译（conditional compilation）支持 `riscv64` 和 `loongarch64` 双平台，因此当你需要编写或阅读平台相关代码时，建议运行相应的脚本以获得最佳体验。

### Code Inspection

项目中包含了名为 `InspectCode.sh` 的代码检查脚本。请确保你定期运行该脚本，以保证代码质量。同时注意 CI 系统的反馈，及时修复代码质量问题。

### Testsuits

为了保持仓库的最小体积，测试套件（testsuites）并未包含在该仓库中。相比之下，当前克隆整个仓库的大小约为 3.3MB，仅克隆最新提交大约为 1.3MB，而单个测试套件大约有 128MB，未来随着测试用例的增加可能会达到 2GB。

但运行和调试内核时，可能需要使用测试套件。你可以从[这里](https://github.com/oscomp/testsuits-for-oskernel/releases/latest)下载测试套件。下载包中已包含 `riscv64` 和 `loongarch64` 平台的测试套件。下载后将其放置在本仓库根目录下，无需解压，构建系统会自动识别并使用正确的测试套件。

### 子项目

#### Kernel annotation bot

Kernel annotation bot 是一个辅助工具，用于帮助你注释（annotate）内核测试。它是一个运行在 GitHub Actions 上的 bot，但你也可以在本地运行它以可视化查看注释信息。当在 GitHub Actions 上运行时，它会分析测试结果，并在提交（commits）中生成注释评论（comment）。在你推送提交后，请注意查看反馈信息。

## 代码检查（Code Inspection）

项目提供 `InspectCode.sh` 脚本，用于检查代码质量。请定期运行该脚本，并及时关注 CI 反馈，修复代码问题。

## 测试套件（Testsuites）

为保持仓库体积精简，测试套件未随源码提供。完整仓库克隆约 3.3MB，仅克隆最新提交约 1.3MB，而单个测试套件已达 128MB，未来可能增加至 2GB。

内核调试和测试需要依赖测试套件。请从 [此链接](https://github.com/oscomp/testsuits-for-oskernel/releases/latest) 下载，包含 `riscv64` 与 `loongarch64` 两个版本。下载后置于仓库根目录，无需解压，构建系统会自动加载对应文件。

## License

本项目（包括内核与所有 crate）遵循 MIT License，详见 [LICENSE](LICENSE)。

部分代码来源于其他项目，受其原始许可协议约束。具体信息记录在相关 crate 的 `lib.rs` 文件中。

### 已引用的第三方代码

- **`path`**：部分实现来自 [.NET Standard Library](https://github.com/dotnet/runtime)，遵循 .NET Foundation 的 MIT 许可证。
- **`TimeSpan`（在 `time` crate 中）**：部分实现同样源自 [.NET Standard Library]，遵循 MIT 许可证。

### 重要声明

未经所有贡献者的书面许可，本项目（包括其派生项目或任何部分）**禁止**被直接用于 [`全国大学生计算机系统能力大赛`](https://os.educg.net) 及类似竞赛。若未使用特定贡献者的代码或引用其实现，则无需取得其单独授权。这是出于学术诚信的考量，在本项目不再用于参赛或毕设等用途后，会放开这部分限制。不过，如果你只是引用其中的一部分库，参考其中的部分代码，或者你的项目不是直接使用该项目或仅进行了非常简单的修改，将不受到本限制，在保留版权信息的情况下，MIT 许可证仍然适用。

此限制适用于本仓库的所有提交记录，包括加入本声明前的提交。除上述竞赛用途限制外，其余使用场景遵循 MIT 许可证条款。

本的最新版本声明始终适用于任何项目以及 forks，查看 [`caiyi/bakaos`](https://github.com/caiyih/bakaos) 的 `README.md` 以获取最新版本。

#### 附加说明

本仓库中特定声明或限制下的内容，用户依然可以根据原始开源协议自由使用、修改和再分发。这些特殊声明仅限制特定竞赛等场景，不影响开源协议允许的其他合法用途。

##### 不受限制的组件

以下内容不受竞赛使用限制，可在相应协议下自由使用：

- **TftpServer**：不受限制，详见代码头部注释。遵循 Microsoft Public License。
- **初赛结果可视化脚本 (`test_preliminary/visualize_result.py`)**：不受限制，遵循 MIT License。
- **内核异常栈回溯脚本 (`kernel/unwinder.py`)**：不受限制，遵循 MIT License。

## Funky!

![9](docs/assets/9.gif)
