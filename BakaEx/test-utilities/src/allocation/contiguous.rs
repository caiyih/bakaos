use core::alloc::Layout;
use std::sync::Arc;

use abstractions::IUsizeAlias;
use address::PhysicalAddress;
use allocation::FrameAllocator;
use allocation_abstractions::IFrameAllocator;
use hermit_sync::SpinMutex;
use mmu_abstractions::IPageTable;

use crate::{allocation::ITestFrameAllocator, memory::TestMMU};

pub struct TestFrameAllocator {
    inner: FrameAllocator,
    native_ptr: *mut u8,
    layout: Layout,
}

impl TestFrameAllocator {
    pub fn new(memory_size: usize) -> Arc<SpinMutex<TestFrameAllocator>> {
        let (native_ptr, layout) = unsafe { alloc_memory(memory_size) };

        let inner = FrameAllocator::new(
            PhysicalAddress::from_usize(native_ptr as usize + layout.size()),
            PhysicalAddress::from_usize(native_ptr as usize),
        );

        Arc::new(SpinMutex::new(TestFrameAllocator {
            inner,
            native_ptr,
            layout,
        }))
    }

    pub fn new_with_mmu(
        memory_size: usize,
    ) -> (
        Arc<SpinMutex<dyn IFrameAllocator>>,
        Arc<SpinMutex<dyn IPageTable>>,
    ) {
        let alloc = Self::new(memory_size);

        (alloc.clone(), TestMMU::new(alloc))
    }
}

impl ITestFrameAllocator for TestFrameAllocator {
    fn check_paddr(&self, paddr: PhysicalAddress, len: usize) -> bool {
        return self.inner.bottom() <= paddr && paddr + len <= self.inner.top();
    }
}

impl IFrameAllocator for TestFrameAllocator {
    fn alloc_frame(&mut self) -> Option<allocation_abstractions::FrameDesc> {
        self.inner.alloc_frame()
    }

    fn alloc_frames(&mut self, count: usize) -> Option<Vec<allocation_abstractions::FrameDesc>> {
        self.inner.alloc_frames(count)
    }

    fn alloc_contiguous(
        &mut self,
        count: usize,
    ) -> Option<allocation_abstractions::FrameRangeDesc> {
        self.inner.alloc_contiguous(count)
    }

    fn dealloc(&mut self, frame: allocation_abstractions::FrameDesc) {
        self.inner.dealloc(frame);
    }

    fn dealloc_range(&mut self, range: allocation_abstractions::FrameRangeDesc) {
        self.inner.dealloc_range(range)
    }
}

impl Drop for TestFrameAllocator {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(self.native_ptr, self.layout);
        }
    }
}

unsafe fn alloc_memory(size: usize) -> (*mut u8, Layout) {
    let layout = Layout::from_size_align(size, constants::PAGE_SIZE).unwrap();
    let ptr = std::alloc::alloc(layout);

    (ptr, layout)
}
