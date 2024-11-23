# Baka OS

## Continuous Integration

| Workflow | Status |
|:---------|:-------|
| Sync to GitLab | [![Sync to GitLab](https://github.com/caiyih/bakaos/actions/workflows/sync.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/sync.yml) |
| Vendor Dependencies | [![Vendor Dependencies for GitLab](https://github.com/caiyih/bakaos/actions/workflows/vendor.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/vendor.yml) |
| Crates Code Quality | [![Crates Code Quality](https://github.com/caiyih/bakaos/actions/workflows/crates-fmt.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/crates-fmt.yml) |
| Crates Tests | [![Crates Tests](https://github.com/caiyih/bakaos/actions/workflows/crates-tests.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/crates-tests.yml) |
| Kernel Code Quality | [![Kernel Code Quality](https://github.com/caiyih/bakaos/actions/workflows/kernel-fmt.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/kernel-fmt.yml) |
| Kernel Tests | [![Kernel CI](https://github.com/caiyih/bakaos/actions/workflows/kernel.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/kernel.yml) |
| Preliminary Grading | [![Preliminary test](https://github.com/caiyih/bakaos/actions/workflows/preliminary.yml/badge.svg)](https://github.com/caiyih/bakaos/actions/workflows/preliminary.yml) |

## GitLab repository of the contest

- [T202410145994289/oskernel2024-9](https://gitlab.eduxiji.net/T202410145994289/oskernel2024-9)

## GitHub repository

- [caiyih/bakaos](https://github.com/caiyih/bakaos)

The GitHub repository is the real repository where the development happens. The GitLab repository is only used for the contest.

## Repo introduction

**IMPORTANT: For detailed documentations, please refer to the `docs` directory.**

This repository contains mainly three parts, `kernel`, `crates` and `test_preliminary`.

**For preliminary test related information, please refer to the `README.md` from the `tests_preliminary` directory.**

If you are viewing vendored branch from gitlab, there is also a `third_party` directory, which contains some third party code that the kernel depends on. 

This is directory is generated automatically by a iced frog.

You should never modify it manually.

The vendor operation is intended to speed up(and prevent failure) the build process for the contest, so only gitlab contains these branches.

### `kernel`

The `kernel` directory is where the kernel source exists. 

For kernel development, you should open your editor/language server's workspace to `kernel` folder instead of the repo root. Otherwise you may encounter errors like `can't find crate for 'test'`.

There is a `Makefile` in this folder, which contains a set of useful commands to build, run and test the kernel.

#### Commands

##### `build`

This build the kernel with debug symbol and no optimization

```bash
$ make build
```

Equivalent to `cargo build`

##### `strip`

Remove all debug symbol of the built artifact.

```bash
$ make strip
```

##### `run`

Build, strip and then run the kernel in QEMU.

```bash
$ make run
```

##### `debug`

Build the kernel with debug symbol and run it in QEMU with GDB server enabled.

```bash
$ make debug
```

You have to connect use a GDB client or run `make connect` to connect to the GDB server.

Also, vscode debugging is supported. Just open the `kernel` folder in vscode and press `F5`.

##### Logging

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

Please note that `OFF` will disable all logging from the `log` crate, but the kernel may still print some messages to the console. But that should not be a thing to worry about.

### `crates`

The `crates` directory contains some code that the kernel directly depends on. These code are implemented in separate crates and can therefore be tested separately even on host machine instead of in the kernel.

All crates are registered in a cargo workspace, so you just have to open your editor/language server in the `crates` folder to edit all crates.

You can run `cargo test --all` in the folder to test all crates.

## License

This project(including kernel and crates) is licensed under the MIT license. See [LICENSE](LICENSE) for more details.

Some code are derived from other projects, and they are licensed under their own licenses. The `lib.rs` file of those crates should contain the license information.

For now, the following crates are derived from other projects:

- `path`: derived from [.NET standard library](https://github.com/dotnet/runtime) and licensed to .NET Foundation under MIT license.

- `TimeSpan` struct from `time`: Partially derived from [.NET standard library](https://github.com/dotnet/runtime) and licensed to .NET Foundation under MIT license.

## Funky!

![9](docs/assets/9.gif)
