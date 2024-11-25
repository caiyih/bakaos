use core::future::Future;

use alloc::collections::VecDeque;
use async_task::{Runnable, ScheduleInfo, WithInfo};
use hermit_sync::{Lazy, SpinMutex};

static mut TASK_SCHEDULER: Lazy<Scheduler> = Lazy::new(Scheduler::new);

struct Scheduler {
    tasks: SpinMutex<VecDeque<Runnable>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: SpinMutex::new(VecDeque::new()),
        }
    }
    pub fn push_back(&self, runnable: Runnable) {
        self.tasks.lock().push_back(runnable);
    }

    pub fn push_front(&self, runnable: Runnable) {
        self.tasks.lock().push_front(runnable);
    }

    pub fn fetch_next(&self) -> Option<Runnable> {
        self.tasks.lock().pop_front()
    }
}

pub fn spawn<TFut, TRet>(task: TFut)
where
    TFut: Future<Output = TRet> + Send + 'static,
    TRet: Send + 'static,
{
    let schedule = move |task: Runnable, info: ScheduleInfo| {
        if info.woken_while_running {
            unsafe { TASK_SCHEDULER.push_back(task) };
        } else {
            unsafe { TASK_SCHEDULER.push_front(task) };
        }
    };

    let spawned = async_task::spawn(task, WithInfo(schedule));
    spawned.0.schedule();
    spawned.1.detach(); // prevent task from being dropped
}

pub fn run_tasks() {
    while let Some(task) = unsafe { TASK_SCHEDULER.fetch_next() } {
        task.run();
    }

    // Although all page tables are mapped with kernel's area.
    // Somehow there's always a bug that causes page fault when back to kernel's control flow.
    // So re-activate kernel's page table here. It just works somehow.
    unsafe { paging::page_table::get_kernel_page_table().activate() };
}

pub fn has_task() -> bool {
    unsafe { !TASK_SCHEDULER.tasks.lock().is_empty() }
}
