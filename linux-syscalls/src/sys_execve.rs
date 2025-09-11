use abstractions::IUsizeAlias;
use address::VirtualAddress;
use constants::ErrNo;
use linux_loader::auxv::AuxVecValues;
use linux_loader::{IExecSource, LinuxLoader, ProcessContext, RawMemorySpace};
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

    /// Perform an execve-like replacement of the current task's address space with a new executable.
    ///
    /// Attempts to load `executable` at `pathname` into a fresh memory space, replace the process's
    /// memory space with the loaded image, initialize the task's trap context (entry PC, stack top,
    /// argv/envp bases and argc), and mark the task Ready. On loader failure this returns
    /// `ErrNo::ExecFormatError`.
    ///
    /// Note: argv and envp parameters are accepted by this function but are currently not wired into
    /// the loader (FIXME). Auxv values are also supplied as defaults (TODO: populate machine info).
    ///
    /// Parameters:
    /// - `executable`: an object implementing `IExecSource` that provides the raw executable bytes.
    /// - `pathname`: the path string used for loader semantics and /proc visibility.
    /// - `argv`: program arguments (currently not forwarded to the loader).
    /// - `envp`: environment variables (currently not forwarded to the loader).
    ///
    /// Returns:
    /// - `Ok(0)` on success.
    /// - `Err(ErrNo::ExecFormatError)` if the loader rejects the executable format.
    ///
    /// Side effects:
    /// - Replaces the process memory space via `process.execve(...)`.
    /// - Updates the task's trap context and status to `TaskStatus::Ready`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Given a `ctx: SyscallContext`, an executable `exe` and path:
    /// let _ = ctx.sys_execve_internal(exe, "/bin/app", &["app", "--help"], &[]);
    /// ```
    #[expect(unused)]
    fn sys_execve_internal(
        &self,
        executable: impl IExecSource,
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

        let memory_space: RawMemorySpace = (mmu, alloc); // FIXME: should be the new process's

        let loader = LinuxLoader::from_raw(
            &executable,
            pathname,
            process_ctx,
            AuxVecValues::default(), // TODO: populate machine info
            self.kernel.fs().lock().clone(),
            &memory_space,
            None, // FIXME: should be the calling thread's
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
