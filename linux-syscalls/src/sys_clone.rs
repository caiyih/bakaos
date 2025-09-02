use abstractions::IUsizeAlias;
use address::{IAddressBase, VirtualAddress};
use task_abstractions::flags::TaskCloneFlags;

use crate::{SyscallContext, SyscallResult};

impl SyscallContext {
    pub fn sys_clone(&self, flags: TaskCloneFlags, stack_top: VirtualAddress) -> SyscallResult {
        let forked = match flags.contains(TaskCloneFlags::THREAD) {
            true => self.task.fork_thread(),
            false => self.task.fork_process(),
        };

        let tid = forked.tid();

        if !stack_top.is_null() {
            forked
                .trap_context_mut()
                .set_stack_top(stack_top.as_usize());
        }

        let process = self.task.linux_process();

        process.push_thread(forked);

        // TODO: add forked task to self.kernel.scheduler

        Ok(tid as isize)
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use kernel_abstractions::IKernel;
    use test_utilities::{kernel::TestKernel, task::TestProcess};

    use super::*;

    fn setup_env() -> (Arc<dyn IKernel>, SyscallContext) {
        let kernel = TestKernel::new().build();

        let (_, task) = TestProcess::new().build();

        let ctx = SyscallContext::new(task, kernel.clone());

        (kernel, ctx)
    }

    #[test]
    fn test_thread_forked() {
        let (_, ctx) = setup_env();

        let ret = ctx.sys_clone(TaskCloneFlags::THREAD, VirtualAddress::null());

        assert!(ret.is_ok());

        let process = ctx.task.linux_process();

        assert_eq!(process.threads().len(), 2);
    }
}
