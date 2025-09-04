use abstractions::IUsizeAlias;
use address::VirtualAddress;
use constants::ErrNo;
use linux_loader::auxv::AuxVecValues;
use linux_loader::{ILoadExecutable, LinuxLoader, ProcessContext};
use platform_specific::ITaskContext;
use platform_specific::TaskTrapContext;
use task_abstractions::status::TaskStatus;

use crate::{SyscallContext, SyscallResult};

impl SyscallContext {
    pub fn sys_execve(
        &self,
        _pathname: VirtualAddress,
        _argv: VirtualAddress,
        _envp: VirtualAddress,
    ) -> SyscallResult {
        todo!()
    }

    /// Replace the current task's address space with a new executable image and prepare its trap
    /// context so the task resumes execution at the new program entry.
    ///
    /// This performs the core work of execve: it constructs a LinuxLoader for `executable` and the
    /// given `pathname`, invokes the process's `execve` to swap in the loader's memory space,
    /// updates the task's trap context (entry PC, stack top, argc, argv/envp bases) and sets the
    /// task status to `Ready`.
    ///
    /// On loader construction failure this returns `ErrNo::ExecFormatError`. On success it returns
    /// `Ok(0)`.
    ///
    /// Note: argv/envp and auxiliary vector population are not yet fully wired — the loader is
    /// currently created with a default `AuxVecValues` and `ProcessContext::new()`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Illustrative example (uses test doubles in real tests).
    /// // let ctx: SyscallContext = ...;
    /// // let exe: impl ILoadExecutable = ...;
    /// // let argv = ["prog", "arg1"];
    /// // let envp = ["KEY=val"];
    /// // let res = ctx.sys_execve_internal(exe, "/bin/prog", &argv, &envp);
    /// // assert_eq!(res.unwrap(), 0);
    /// ```
    #[expect(unused)]
    fn sys_execve_internal(
        &self,
        executable: impl ILoadExecutable,
        pathname: &str,
        argv: &[&str],
        envp: &[&str],
    ) -> SyscallResult {
        let process = self.task.linux_process();

        let (mmu, alloc) = {
            let mem = process.memory_space().lock();
            (mem.mmu().clone(), mem.allocator().clone())
        };

        let mut process_ctx = ProcessContext::new();

        // FIXME: Pass argv, envp

        // TODO: resolve machine's information and pass it to auxv

        let loader = LinuxLoader::from_raw(
            &executable,
            pathname,
            process_ctx,
            AuxVecValues::default(), // FIXME
            self.kernel.fs().lock().clone(),
            mmu,
            alloc,
        )
        .map_err(|_| ErrNo::ExecFormatError)?;

        let calling_thread = self.task.tid();

        process.execve(loader.memory_space, calling_thread);

        let trap_ctx = TaskTrapContext::new(
            loader.entry_pc.as_usize(),
            loader.stack_top.as_usize(),
            loader.ctx.argv.len(),
            loader.argv_base.as_usize(),
            loader.envp_base.as_usize(),
        );

        self.task.trap_context_mut().copy_from(&trap_ctx);

        self.task.update_status(TaskStatus::Ready);

        Ok(0)
    }
}
