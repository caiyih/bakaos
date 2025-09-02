use alloc::sync::Arc;
use hermit_sync::SpinMutex;
use task_abstractions::{ITaskIdAllocator, TaskId};

pub struct TaskIdAllocator {
    current: SpinMutex<u32>,
}

impl TaskIdAllocator {
    pub fn new(base: u32) -> Arc<Self> {
        Arc::new(Self {
            current: SpinMutex::new(base),
        })
    }
}

impl ITaskIdAllocator for TaskIdAllocator {
    fn alloc(self: Arc<Self>) -> TaskId {
        let mut current = self.current.lock();
        let id = *current;

        *current += 1;

        TaskId::new(id, self.clone())
    }

    fn dealloc(self: Arc<Self>, _task_id: u32) {}
}
