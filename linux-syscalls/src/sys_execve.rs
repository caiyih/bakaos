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
