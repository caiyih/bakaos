# Continuous Integration

本项目使用持续集成来保持代码质量高并防止回归。每次推送都会被 CI 系统检查和测试，确保代码始终高质量和稳定。

共有以下 10 个工作流：

1. Sync to GitLab / sync

该工作流在每次推送时将代码同步到 GitLab，使得 GitLab 仓库与 GitHub 仓库保持一致。

2. Vendor Dependencies for GitLab / vendor

该工作流在每次推送时将内核所有依赖储存到 `<branch>-vendored` 分支，并推送到 GitLab 仓库，保持 GitHub 仓库干净，同时加速比赛时的构建过程（避免网络问题导致的构建失败）。

3. Crates Code Quality / Code Lint

该工作流在每次推送时检查 crates 代码的质量，确保代码符合软件工程的最佳实践。

5. Crates Code Quality / Code Format

该工作流在每次推送时检查 crates 代码的格式，确保代码风格良好统一。

4. Crates Tests / Run Tests

该工作流在每次推送时运行 crates 的测试，确保代码的正确性，避免发生回归。

6. Kernel Code Quality / Code Lint

该工作流在每次推送时检查内核代码的质量，确保内核代码符合软件工程的最佳实践。

7. Kernel Code Quality / Code Format

该工作流在每次推送时检查内核代码的格式，确保内核代码风格良好统一。

8. Kernel CI / Build Only

该工作流在每次推送时构建内核，确保内核能够成功编译。

9. Kernel CI / Run

该工作流在每次推送时运行内核，确保内核能够正常运行，避免发生回归。

10. Preliminary test / Grading

该工作流在每次推送时运行初赛测试，确保内核能够通过初赛测试，避免发生回归。

所有工作流记录可以在 GitHub 仓库的 [Actions](https://github.com/caiyih/bakaos/actions) 页面查看。也可以在 GitHub 网页的每次推送的 commit 页面查看工作流的运行情况，便于追踪代码的变化和 CI 的结果。
