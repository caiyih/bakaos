# Baka OS

| English | [简体中文](./README.zh-cn.md) |

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

> *You might be looking for [BakaOS](https://github.com/caiyih/bakaos/tree/bakaos/master), our previous kernel, which has been moved to the `BakaOS/master` branch for separate development. Although BakaOS is more functional, BakaEx has some exciting features that led us to abandon BakaOS. BakaEx adopts our proposed next-generation kernel architecture, which has many better features and fixes some design errors and legacy issues in BakaOS.*

BakaEx (BakaOS Extended), the successor to BakaOS, is a Rust-based UNIX-like asynchronous operating system kernel that supports RISC-V and LoongArch64 architectures.

The kernel is organized into multiple crates, strictly following the **dual-crate implementation pattern**: each functional domain is split into an _abstractions_ crate (pure interfaces) and an implementation crate. This makes every kernel subsystem **highly decoupled, testable, and replaceable**.

Most kernel code including syscalls, components like filesystem, memory space and more — can be tested **directly on the host machine** without running on bare-metal. This is achieved through:

- **Inversion of Control (IoC)**: clear separation of interfaces and implementations
- **Dependency Injection**: all critical services are passed explicitly, avoiding global state
- **Platform-independent implementations**: enabling host-based execution for unit tests
- **MMU simulation**: simulates a whole isolated virtual memory space with read/write capability

Only the **platform abstraction layer** and a minimal part of the core kernel require bare-metal execution. Everything else shares the same code in both the production kernel and host-based tests, ensuring consistency and reliability.

Key advantages of this design include:

- **Systematic workflow**: complete development–testing–validation loop
- **Practical focus**: solves efficiency bottlenecks in real-world kernel development
- **Long-term extensibility**: seamless integration with formal verification and cross-architecture testing
- **Industrial-grade code quality**: Mock + host-based testing ensures modular, highly testable, and regression-friendly code
- **Platform independence**: develop and test on Windows, Linux, or macOS without QEMU or bare-metal setup
- **Decoupled and pluggable architecture**: freely combine, replace, or remove components for specialized needs
- **Parallelized testing**: run `cargo test` with full parallelization to greatly accelerate testing cycles
- **Low entry barrier**: development process is close to standard Rust application development, suitable even for teaching
- **Unikernel compatibility**: the kernel design is a superset of the Unikernel model
- **Flexible isolation environment**: through dependency injection and IKernel instantiation mechanism, the OS can easily create and modify kernel-level isolated running environments.

Our design philosophy is **"logically microkernel, physically monolithic kernel"**.  
We combine the **maintainability of microkernels** with the **performance of monolithic kernels** by keeping subsystems logically independent yet running in the same address space.

For a detailed explanation of BakaEx’s architecture and testing capabilities, see:

- [BakaEx Documentation](docs/BakaEx/README.md) （Avaliable in Simplified Chinese）

*Many contents are still BakaOS’ legacy (including the README content, but BakaEx’s documentation is not included), we will clean them up soon. We will also migrate BakaOS’ excellent features to BakaEx as soon as possible.*

## Continuous Integration

This repository uses continuous integration to keep the code quality high and prevent regressions. Every push is inspected and tested by the CI system, ensuring that the code is always high quality and stable.

| Workflow            | Status                                                                                                                                                                   |
| :------------------ | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Crates Code Quality | [![Crates Code Quality](https://github.com/caiyih/bakaos/actions/workflows/crates-fmt.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/crates-fmt.yml) |
| Crates Tests        | [![Crates Tests](https://github.com/caiyih/bakaos/actions/workflows/crates-tests.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/crates-tests.yml)    |
| Kernel Code Quality | [![Kernel Code Quality](https://github.com/caiyih/bakaos/actions/workflows/kernel-fmt.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/kernel-fmt.yml) |
| Kernel Tests        | [![Kernel CI](https://github.com/caiyih/bakaos/actions/workflows/kernel.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/kernel.yml)                   |
| Preliminary Grading | [![Preliminary test](https://github.com/caiyih/bakaos/actions/workflows/preliminary.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/preliminary.yml)  |

<!-- | Sync to GitLab | [![Sync to GitLab](https://github.com/caiyih/bakaos/actions/workflows/sync.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/sync.yml) |
| Vendor Dependencies | [![Vendor Dependencies for GitLab](https://github.com/caiyih/bakaos/actions/workflows/vendor.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/vendor.yml) | -->

<!--
## GitLab repository of the contest

- [T202410145994289/oskernel2024-9](https://gitlab.eduxiji.net/T202410145994289/oskernel2024-9)

## GitHub repository

- [caiyih/bakaos](https://github.com/caiyih/bakaos)

The GitHub repository is the real repository where the development happens. The GitLab repository is only used for the contest. -->

## Development

To develop this project, the following tools are required:

- Cargo and Rust: This projects uses a specified versoin of Rust, which is specified in the `rust-toolchain.toml` file. When cargo is installed, it will automatically download the specified version of Rust.

And that's it, so you can complie the kernel only by running `cargo build` in the root directory.

If the lwext4 is enabled, you need to install the following tools:

- loongarch64-linux-musl-cc: for building lwext4 for LoongArch64
- riscv64-linux-musl-cc: for building lwext4 for RISC-V64

You can simply disable the lwext4 feature if you don't need it. We are considering migrate to completely Rust implemented ext4 filesystem to avoid the dependency on the C library.

## Repo introduction

**IMPORTANT: For detailed documentations, please refer to the `docs` directory.**

This repository contains mainly three parts, `kernel`, `crates` and other subprojects.

<!-- **For preliminary test related information, please refer to the `README.md` from the `tests_preliminary` directory.** -->

<!--
If you are viewing vendored branch from gitlab, there is also a `third_party` directory, which contains some third party code that the kernel depends on.

This is directory is generated automatically by a iced frog.

You should never modify it manually.

The vendor operation is intended to speed up(and prevent failure) the build process for the contest, so only gitlab contains these branches. -->

### `kernel`

The `kernel` directory is where the kernel source exists.

<!--
For kernel development, you should open your editor/language server's workspace to `kernel` folder instead of the repo root. Otherwise you may encounter errors like `can't find crate for 'test'`. -->

There is a build system in `Makefile`, type `make help` for detailed usage.

_Note it's the file in the kernel directory, not the one in the root directory._

```
BakaOS build system
Usage:
- make help        : Show this help message
- make build       : Build os kernel
- make run         : Run os kernel with qemu
- make debug       : Launch the os with qemu and wait for gdb
- make connect     : Launch gdb and connect to qemu
- make clean       : Clean the build artifacts
- make stacktrace  : Parse and generate the stacktrace of qemu output
- make symboltable : Generate symbol table of current elf file at kernel/.disassembled
- make strip       : Strip the kernel elf file, usually this is unnecessary

Environment variables:
- ARCH=riscv64(default)|loongarch64            : Set the target architecture
- MODE=debug(default)|release-with-debug       : Set the build mode
- LOG=TRACE|DEBUG|INFO(default)|WARN|ERROR|OFF : Set the log level
```

The build artifacts of the kernel are located in `target/<ARCH_TRIPLET>/<MODE>/bakaos`.

The basic usages are listed above. But there's also some precations.

#### Envrionment variables

The envrionment variables are used to control the build process. Currently, the following variables are supported:

- `ARCH`: Set the target architecture. Default is `riscv64`, `loongarch64` is also supported.

- `MODE`: Set the build mode. Default is `debug`. Supported values are `debug`, `release-with-debug` and `release`.

- `LOG`: Set the log level. Default is `INFO`. See info below for more details.

#### Publish profile

Cargo supports `debug` and `release` profiles, which is also parts of our build system. But our build system also provides another profile named `release-with-debug`.

The `release-with-debug` profile is used to build the kernel with release level optimization but with debug symbols, which is useful for debugging bugs only happens in release mode.

#### Debugging

To run the kernel with gdb enabled, simply run `make debug` command.

Build the kernel with debug symbol and run it in QEMU with GDB server enabled.

```bash
$ make debug
```

You have to connect use a GDB client or run `make connect` to connect to the GDB server.

Also, vscode debugging is supported. Ensure you have installed the [`CodeLLDB`](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) extension. Just open the development workspace in vscode and press `F5`.

#### Clean

To clean the build artifacts, simply run `make clean` command. Since the kernel is a separate workspace relative to `crates`, this does NOT clean the build artifacts of `crates`.

#### Stacktrace

To parse and generate the stacktrace of qemu output, simply run `make stacktrace` command. Usually you don't have to run this command manually. But it's useful when you want to reappear the crash report with only the output and kernel binary from somewhere.

The CI workflows will upload the kernel binary and outputs to the CI artifacts. When the kernel panics in CI, you can download the artifacts to reproduce the crash report.

#### Logging

The kernel uses the `log` crate for logging. You can set the `LOG` environment variable to control the log level.

eg:

```bash
$ make run LOG=TRACE
```

This runs the kernel with log level set to `TRACE`.

Please note that the log level is hard coded at compile time. But you don't have to worry as `run` command will rebuild the kernel with the specified log level.

There are 6 log levels in total:

- `ERROR`
- `WARN`
- `INFO`
- `DEBUG`
- `TRACE`
- `OFF`

Level `ERROR` is the highest level, and `TRACE` is the lowest level.

The default log level is `INFO`.

Please note that `OFF` will disable all logging from the `log` crate, but the kernel may still print negligible messages to the console. But that should not be a thing to worry about.

### `crates`

The `crates` directory contains some code that the kernel directly depends on. These code are implemented in separate crates and can therefore be tested separately even on host machine instead of in the kernel.

All crates are registered in a cargo workspace, so you just have to open your editor/language server in the `crates` folder to edit all crates.

#### Hardware abstraction layer

We've developed a hardware abstraction layer for the kernel to abstract the hardware details. It supports both `riscv64` and `loongarch64` and can be easily extended to support more platforms. It's consist of the following crates:

- `platform-abstractions`: The most underlying crate that provides the basic hardware abstraction. This crates provides boot and interrupt handling job. The boot part enables virtual memory, sets up the higher half kernel space, and doing some platform dependent initialization and then jumps directly to the kernel code. The trap part handles the interrupt in a coroutine way, which means when interrupt happens, it saves the current context, and the returns to the code where you enter the user space. This allows the kernel schedule tasks in asnychronous way.

- `platform-specific`: The platform specific crate that provides platform specific syscall ids, trap context, serial IO, access the platform specific registers(including general purpose registers and some CSRs), processor core id, and ability to translate virtual address to physical address.

- `drivers`: The drivers crate that provides the machine instance abstraction and interfaces to access the platform specific hardware. This includes the RTC access, performance counter, and block device...

- `page_table`: The platform independent page table abstraction. It's used manage the virtual memory with paging mechanism. This crates uses agressive inlining, constant propagation and branch elimination to achieve almost zero overhead(With some features enabled).

### Workspace

It's highly recommended to develop this project with Visual Studio Code. We've provided some scripts to generate a development workspace for you:

- `SetupRV64Workspace.sh`
- `SetupLA64Workspace.sh`

When you run these scripts, it will generate a development workspace for you at the cwd where you run the script.

This project uses conditional compilation to support both `riscv64` and `loongarch64`. So when you want to write/read platform specific code, you should run the corresponding script for best experience.

### Code Inspection

There is a script named `InspectCode.sh` to help you inspect the code quality. Ensure you run this script from time to time. Also pay attention to CI feedback and fix code quality issues in time.

### Testsuits

The testsuites are not contained in this repository to maintain the minimal size of the repository. In comparison, it takes only about 3.3MB if you clone the whole repository and only 1.3MB if only the latest commit, while each testsuits takes about 128MB and may be about 2GB in the future with more tests added.

But to run and debug the kernel may requires the testsuites. You can download the testsuites from [here](https://github.com/oscomp/testsuits-for-oskernel/releases/latest). Both the ones for `riscv64` and `loongarch64` are included, you should download them and place them in the root of this repository and do NOT decompress them, the build system will automatically pick up the correct one.

## License

This project (including the kernel and crates) is licensed under the MIT License. See [LICENSE](LICENSE) for more details.

Some code within this project is derived from other projects and is subject to their respective licenses. The `lib.rs` file of each relevant crate includes the corresponding license information.

Currently, the following crates include code derived from other projects:

- **`path`**: Derived from the [.NET Standard Library](https://github.com/dotnet/runtime), licensed under the MIT License by the .NET Foundation.

- **`TimeSpan` struct in `time`**: Partially derived from the [.NET Standard Library](https://github.com/dotnet/runtime), licensed under the MIT License by the .NET Foundation.

## Funky!

![9](docs/assets/9.gif)
