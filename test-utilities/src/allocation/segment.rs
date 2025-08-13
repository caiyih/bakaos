use core::{alloc::Layout, ptr::NonNull};
use std::{
    alloc::{AllocError, Allocator},
    collections::BTreeMap,
    sync::Arc,
};

use abstractions::IUsizeAlias;
use address::{PhysicalAddress, PhysicalAddressRange};
use allocation_abstractions::{FrameDesc, FrameRangeDesc, IFrameAllocator};
use hermit_sync::SpinMutex;
use mmu_abstractions::IMMU;

use crate::{allocation::ITestFrameAllocator, memory::TestMMU};

pub struct TestFrameAllocator {
    records: BTreeMap<PhysicalAddress, HostMemory>,
}

impl TestFrameAllocator {
    pub fn new() -> Arc<SpinMutex<TestFrameAllocator>> {
        Arc::new(SpinMutex::new(TestFrameAllocator {
            records: BTreeMap::new(),
        }))
    }

    pub fn new_with_mmu() -> (
        Arc<SpinMutex<dyn IFrameAllocator>>,
        Arc<SpinMutex<dyn IMMU>>,
    ) {
        let alloc = Arc::new(SpinMutex::new(TestFrameAllocator {
            records: BTreeMap::new(),
        }));

        (alloc.clone(), TestMMU::new(alloc))
    }
}

impl ITestFrameAllocator for TestFrameAllocator {
    fn check_paddr(&self, paddr: PhysicalAddress, len: usize) -> bool {
        let range = PhysicalAddressRange::from_start_len(paddr, len);

        for mem in self.records.values() {
            let target_range = mem.paddr_range();

            if target_range.intersects(&range) {
                return true;
            }
        }

        return false;
    }

    fn linear_map(&self, _: PhysicalAddress) -> Option<*mut u8> {
        None
    }
}

pub(crate) struct HostMemory {
    pub ptr: NonNull<u8>,
    pub layout: Layout,
}

impl HostMemory {
    pub fn alloc(num_frames: usize) -> (PhysicalAddress, Self) {
        let layout = create_layout(num_frames);
        let (pa, ptr) = heap_allocate(layout).unwrap();

        (pa, Self { ptr, layout })
    }

    pub fn paddr(&self) -> PhysicalAddress {
        PhysicalAddress::from_usize(self.ptr.as_ptr() as usize)
    }

    pub fn paddr_range(&self) -> PhysicalAddressRange {
        PhysicalAddressRange::from_start_len(self.paddr(), self.layout.size())
    }
}

impl Drop for HostMemory {
    fn drop(&mut self) {
        heap_deallocate(self.ptr, self.layout);
    }
}

impl IFrameAllocator for TestFrameAllocator {
    fn alloc_frame(&mut self) -> Option<allocation_abstractions::FrameDesc> {
        let (pa, mem) = HostMemory::alloc(1);

        self.records.insert(pa, mem);

        Some(unsafe { FrameDesc::new(pa) })
    }

    fn alloc_frames(&mut self, count: usize) -> Option<Vec<allocation_abstractions::FrameDesc>> {
        let mut v = Vec::with_capacity(count);

        for _ in 0..count {
            v.push(self.alloc_frame()?);
        }

        Some(v)
    }

    fn alloc_contiguous(
        &mut self,
        count: usize,
    ) -> Option<allocation_abstractions::FrameRangeDesc> {
        let (pa, mem) = HostMemory::alloc(count);

        self.records.insert(pa, mem);

        Some(unsafe { FrameRangeDesc::new(pa, count) })
    }

    fn dealloc(&mut self, frame: allocation_abstractions::FrameDesc) {
        self.records.remove(&frame.0);
        core::mem::forget(frame);
    }

    fn dealloc_range(&mut self, range: allocation_abstractions::FrameRangeDesc) {
        self.records.remove(&range.start);
        core::mem::forget(range);
    }
}

const fn create_layout(num_frame: usize) -> Layout {
    unsafe {
        Layout::from_size_align_unchecked(constants::PAGE_SIZE * num_frame, constants::PAGE_SIZE)
    }
}

fn heap_allocate(layout: Layout) -> Result<(PhysicalAddress, NonNull<u8>), AllocError> {
    let slice_ptr: NonNull<[u8]> = std::alloc::Global.allocate_zeroed(layout)?;

    let raw_ptr = slice_ptr.as_ptr() as *mut u8;

    Ok((PhysicalAddress::from_usize(raw_ptr as usize), unsafe {
        NonNull::new_unchecked(raw_ptr)
    }))
}

fn heap_deallocate(ptr: NonNull<u8>, layout: Layout) {
    unsafe { std::alloc::Global.deallocate(ptr, layout) }
}
