use threading::yield_now;

use crate::{SyscallContext, SyscallResult};

impl SyscallContext {
    pub async fn sys_sched_yield(&self) -> SyscallResult {
        yield_now().await;

        Ok(0) // according to man page, sched_yield always success
    }
}

#[cfg(test)]
mod tests {
    use core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll, Waker},
    };
    use std::sync::Arc;

    use kernel_abstractions::IKernel;
    use task_abstractions::ITask;
    use test_utilities::{kernel::TestKernel, task::TestProcess};

    use super::*;

    fn setup_test_task() -> (Arc<dyn ITask>, Arc<dyn IKernel>) {
        let kernel = TestKernel::new().build();
        let (_, task) = TestProcess::new().build();

        (task, kernel)
    }

    #[test]
    fn test_sys_sched_yield() {
        let (task, kernel) = setup_test_task();
        let ctx = SyscallContext::new(task, kernel);

        let mut fut = ctx.sys_sched_yield();

        let mut ctx = Context::from_waker(Waker::noop());

        let poll1 = Future::poll(unsafe { Pin::new_unchecked(&mut fut) }, &mut ctx);
        let poll2 = Future::poll(unsafe { Pin::new_unchecked(&mut fut) }, &mut ctx);

        assert_eq!(poll1, Poll::Pending);
        assert_eq!(poll2, Poll::Ready(Ok(0)));
    }
}
