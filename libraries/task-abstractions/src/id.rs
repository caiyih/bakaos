use alloc::sync::Arc;

use crate::TaskId;

pub trait ITaskIdAllocator {
    fn alloc(self: Arc<Self>) -> TaskId;

    fn dealloc(self: Arc<Self>, task_id: u32);
}
