use core::cell::UnsafeCell;

use alloc::sync::Arc;
use hermit_sync::SpinMutex;
use linux_task_abstractions::{ILinuxProcess, ILinuxTask};
use platform_specific::TaskTrapContext;
use task_abstractions::{status::TaskStatus, IProcess, ITask, TaskId, UserTaskStatistics};
use trap_abstractions::ITaskTrapContext;

pub struct LinuxTask {
    pub(crate) id: TaskId,
    pub(crate) process: UnsafeCell<Option<Arc<dyn ILinuxProcess>>>,
    pub(crate) inner: SpinMutex<TaskMutableInner>,
    pub(crate) trap_ctx: UnsafeCell<TaskTrapContext>,
}

unsafe impl Send for LinuxTask {}
unsafe impl Sync for LinuxTask {}

impl LinuxTask {
    pub(crate) fn new(id: TaskId, trap_ctx: TaskTrapContext) -> Arc<Self> {
        Arc::new(LinuxTask {
            id,
            process: UnsafeCell::new(None),
            trap_ctx: UnsafeCell::new(trap_ctx),
            inner: SpinMutex::new(TaskMutableInner::default()),
        })
    }
}

pub(crate) struct TaskMutableInner {
    status: TaskStatus,
    stats: UserTaskStatistics,
}

impl Default for TaskMutableInner {
    fn default() -> Self {
        Self {
            status: TaskStatus::Uninitialized,
            stats: UserTaskStatistics::default(),
        }
    }
}

impl Clone for TaskMutableInner {
    fn clone(&self) -> Self {
        Self {
            status: self.status,
            stats: UserTaskStatistics::default(),
        }
    }
}

impl ITask for LinuxTask {
    fn tid(&self) -> u32 {
        *self.id
    }

    fn tgid(&self) -> u32 {
        self.process_ref().pid()
    }

    fn process(&self) -> Arc<dyn IProcess> {
        self.process_ref().clone()
    }

    fn status(&self) -> TaskStatus {
        self.inner.lock().status
    }

    fn stats(&self) -> UserTaskStatistics {
        self.inner.lock().stats.clone()
    }

    fn trap_context(&self) -> &dyn ITaskTrapContext {
        unsafe { self.trap_ctx.get().as_ref().unwrap() }
    }

    fn trap_context_mut(&self) -> &mut dyn ITaskTrapContext {
        unsafe { self.trap_ctx.get().as_mut().unwrap() }
    }

    fn update_status(&self, new_status: TaskStatus) -> TaskStatus {
        let mut inner = self.inner.lock();

        let prev_status = inner.status;
        inner.status = new_status;

        prev_status
    }

    fn fork_thread(&self) -> Arc<dyn ITask> {
        let process = self.process_ref().clone();
        let id = process.alloc_id();

        let mut trap_ctx = TaskTrapContext::default();

        trap_ctx.copy_from(self.trap_context());

        Arc::new(LinuxTask {
            id,
            process: UnsafeCell::new(Some(process)),
            trap_ctx: UnsafeCell::new(trap_ctx),
            inner: SpinMutex::new(self.inner.lock().clone()),
        })
    }

    fn fork_process(&self) -> Arc<dyn ITask> {
        todo!()
    }
}

impl ILinuxTask for LinuxTask {
    fn linux_process(&self) -> Arc<dyn ILinuxProcess> {
        self.process_ref().clone()
    }
}

impl LinuxTask {
    fn process_ref(&self) -> &Arc<dyn ILinuxProcess> {
        unsafe { self.process.get().as_ref().unwrap().as_ref().unwrap() }
    }
}
