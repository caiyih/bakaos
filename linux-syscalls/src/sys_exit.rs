use task_abstractions::status::TaskStatus;

use crate::{SyscallContext, SyscallResult};

impl SyscallContext {
    pub fn sys_exit(&self, code: u8) -> SyscallResult {
        self.task.update_status(TaskStatus::Exited);
        *self.task.process().exit_code().lock() = Some(code);

        Ok(code as isize)
    }
}

#[cfg(test)]
mod tests {
    use test_utilities::{kernel::TestKernel, task::TestProcess};

    use super::*;

    fn setup_env() -> SyscallContext {
        let kernel = TestKernel::new().build();
        let (_, task) = TestProcess::new().build();

        SyscallContext::new(task, kernel)
    }

    #[test]
    fn test_no_exit_code_before_call() {
        let ctx = setup_env();

        assert_eq!(*ctx.task.process().exit_code().lock(), None);
    }

    fn test_exit_code_received(exit_code: u8) {
        let ctx = setup_env();

        let ret = ctx.sys_exit(exit_code);

        assert_eq!(ret, Ok(exit_code as isize));
        assert_eq!(*ctx.task.process().exit_code().lock(), Some(exit_code));
    }

    #[test]
    fn test_exit_code_received_normal() {
        test_exit_code_received(0);
    }

    #[test]
    fn test_exit_code_received_with_abnormal_code() {
        test_exit_code_received(42);
    }

    #[test]
    fn test_status_updated() {
        let ctx = setup_env();
        ctx.sys_exit(0).unwrap();

        assert_eq!(ctx.task.status(), TaskStatus::Exited);
    }
}
