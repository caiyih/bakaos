use core::{
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    sync::atomic::Ordering,
    task::{Poll, Waker},
};

use alloc::sync::Arc;
use log::debug;
use tasks::{TaskControlBlock, TaskStatus};

use crate::{
    processor::ProcessorUnit,
    trap::{return_to_user, user_trap_handler_async},
};

struct ExposeWakerFuture;

impl Future for ExposeWakerFuture {
    type Output = Waker;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        ctx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        Poll::Ready(ctx.waker().clone())
    }
}

#[no_mangle]
// Complete lifecycle of a task from ready to exited
async fn task_loop(tcb: Arc<TaskControlBlock>) {
    debug_assert!(
        tcb.is_ready(),
        "task must be ready to run, but got {:?}",
        tcb.task_status
    );

    unsafe {
        // We can't pass the waker(or the context) to nested functions, so we store it in the tcb.
        *tcb.waker.get() = MaybeUninit::new(ExposeWakerFuture.await);
        *tcb.start_time.get().as_mut().unwrap() =
            MaybeUninit::new(crate::timing::current_timespec());
    }

    *tcb.task_status.lock() = TaskStatus::Running;
    while !tcb.is_exited() {
        return_to_user(&tcb);

        // Returned from user program. Entering trap handler.
        // We've actually saved the trap context before returned from `return_to_user`.

        debug_assert!(tcb.is_running(), "task should be running");

        user_trap_handler_async(&tcb).await;
    }

    debug!(
        "Task {} has completed its lifecycle with code: {}, cleaning up...",
        tcb.task_id.id(),
        tcb.exit_code.load(Ordering::Relaxed)
    );

    // Some cleanup, like dangling child tasks, etc.
}

struct TaskFuture<F: Future + Send + 'static> {
    tcb: Arc<TaskControlBlock>,
    fut: F,
}

impl<TFut: Future + Send + 'static> TaskFuture<TFut> {
    fn new(tcb: Arc<TaskControlBlock>, fut: TFut) -> Self {
        Self {
            tcb: tcb.clone(),
            fut,
        }
    }
}

impl<TFut: Future + Send + 'static> Future for TaskFuture<TFut> {
    type Output = TFut::Output;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        ctx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let pinned = unsafe { self.get_unchecked_mut() };
        let cpu = ProcessorUnit::current();
        cpu.stage_task(pinned.tcb.clone());
        let ret = unsafe { Pin::new_unchecked(&mut pinned.fut).poll(ctx) };
        cpu.pop_staged_task();
        ret
    }
}

#[allow(unused)]
pub fn spawn_task(tcb: Arc<TaskControlBlock>) {
    tcb.init();
    let fut = TaskFuture::new(tcb.clone(), task_loop(tcb));
    threading::spawn(fut);
}
