use alloc::vec::Vec;
use hermit_sync::{Lazy, SpinMutex};

pub struct TrackedTaskId(usize, bool);

impl TrackedTaskId {
    /// Allocate a new `TrackedTaskId` bypassing the deallocation mechanism.
    ///
    /// # Safety
    /// This may lead to duplicate `TrackedTaskId`s. So you must be careful
    /// using this function.
    /// Usually, the kernel only allocates initrpoc with this function.
    pub unsafe fn unsafe_allocate(id: usize) -> Self {
        TrackedTaskId(id, false) // do not deallocate
    }

    fn new(id: usize) -> Self {
        TrackedTaskId(id, true)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl Drop for TrackedTaskId {
    fn drop(&mut self) {
        if self.1 {
            unsafe {
                TASK_ID_ALLOCATOR.lock().deallocate(self.0);
            }
        }
    }
}

static mut TASK_ID_ALLOCATOR: SpinMutex<Lazy<TaskIdAllocator>> =
    SpinMutex::new(Lazy::new(TaskIdAllocator::new));

struct TaskIdAllocator {
    // Minimum value of the next TId to be allocated
    current: usize,
    // Recycled that is greater than current
    recycled: Vec<usize>,
}

impl TaskIdAllocator {
    fn new() -> Self {
        TaskIdAllocator {
            // reserve first 2000 TIDs
            current: 2000,
            recycled: Vec::new(),
        }
    }

    fn allocate(&mut self) -> TrackedTaskId {
        // For now, it doesn't really make sense to recycle these pids, as 2^64 is far more than enough.
        // For easier debugging, i decide to disable this temporarily.
        // match self.recycled.pop() {
        //     Some(tid) => TrackedTaskId::new(tid),
        //     None => {
        let tid = self.current;
        self.current += 1;
        TrackedTaskId::new(tid)
        //     }
        // }
    }

    fn deallocate(&mut self, tid: usize) {
        debug_assert!(tid < self.current);
        debug_assert!(
            self.recycled.iter().all(|elem| *elem != tid),
            "tid {} has been deallocated! Current: {}, recycled: {:?}",
            tid,
            self.current,
            self.recycled
        );

        self.recycled.push(tid);
    }
}

pub fn allocate_tid() -> TrackedTaskId {
    unsafe { TASK_ID_ALLOCATOR.lock().allocate() }
}
