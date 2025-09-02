use core::{fmt::Debug, ops::Deref};

use alloc::sync::Arc;

use crate::ITaskIdAllocator;

pub struct TaskId {
    id: u32,
    allocator: Option<Arc<dyn ITaskIdAllocator>>,
}

impl TaskId {
    pub fn new(id: u32, allocator: Arc<dyn ITaskIdAllocator>) -> Self {
        Self {
            id,
            allocator: Some(allocator),
        }
    }

    /// Create a new task id without allocator
    ///
    /// # Safety
    ///
    /// The caller must ensure that the task id is unique.
    pub unsafe fn new_bypass(id: u32) -> Self {
        Self {
            id,
            allocator: None,
        }
    }
}

impl PartialEq for TaskId {
    fn eq(&self, other: &Self) -> bool {
        if self.id != other.id {
            return false;
        }

        if self.allocator.is_some() != other.allocator.is_some() {
            return false;
        }

        if self.allocator.is_none() {
            return true;
        }

        Arc::ptr_eq(
            self.allocator.as_ref().unwrap(),
            other.allocator.as_ref().unwrap(),
        )
    }
}

impl Debug for TaskId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TaskId")
            .field("id", &self.id)
            .field("has_allocator", &self.allocator.is_some())
            .finish()
    }
}

impl Deref for TaskId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl Drop for TaskId {
    fn drop(&mut self) {
        if let Some(allocator) = self.allocator.take() {
            allocator.dealloc(self.id);
        }
    }
}
