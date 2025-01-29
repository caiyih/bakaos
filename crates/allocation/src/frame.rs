use abstractions::IUsizeAlias;
use alloc::vec::Vec;
use core::iter::Iterator;
use core::ops::Drop;
use core::usize;

use address::{IPageNum, PhysicalAddress, PhysicalPageNum, PhysicalPageNumRange};
use hermit_sync::{Lazy, SpinMutex};
use log::debug;

pub struct TrackedFrame(PhysicalPageNum);

impl TrackedFrame {
    fn new(ppn: PhysicalPageNum) -> Self {
        TrackedFrame(ppn)
    }

    pub fn ppn(&self) -> PhysicalPageNum {
        self.0
    }

    pub fn zero(&self) {
        let va = self.ppn().start_addr::<PhysicalAddress>().to_high_virtual();

        let ptr = unsafe { va.as_mut_ptr::<u64>() };

        for i in 0..(constants::PAGE_SIZE / core::mem::size_of::<u64>()) {
            unsafe { ptr.add(i).write_volatile(0) };
        }
    }
}

impl Drop for TrackedFrame {
    fn drop(&mut self) {
        dealloc_frame(self);
    }
}

pub struct TrackedFrameRange {
    start: PhysicalPageNum,
    count: usize,
}

impl TrackedFrameRange {
    pub fn new(start: PhysicalPageNum, count: usize) -> Self {
        TrackedFrameRange { start, count }
    }

    pub fn to_range(&self) -> PhysicalPageNumRange {
        PhysicalPageNumRange::from_start_count(self.start, self.count)
    }
}

impl Drop for TrackedFrameRange {
    fn drop(&mut self) {
        for i in 0..self.count {
            unsafe {
                dealloc_frame_unchecked(self.start + i);
            }
        }
    }
}

trait IFrameAllocator {
    fn alloc_frame(&mut self) -> Option<TrackedFrame>;
    // Allocates `count` frames and returns them as a vector, no guarantee that the frames are contiguous
    fn alloc_frames(&mut self, count: usize) -> Option<Vec<TrackedFrame>>;
    // Allocates `count` frames and returns them as a range, guaranteeing that the frames are contiguous
    fn alloc_contiguous(&mut self, count: usize) -> Option<TrackedFrameRange>;

    fn dealloc(&mut self, frame: &TrackedFrame);

    fn dealloc_multiple(&mut self, frames: impl Iterator<Item = TrackedFrame>) {
        for frame in frames {
            self.dealloc(&frame);
        }
    }
}

static mut FRAME_ALLOCATOR: SpinMutex<Lazy<FrameAllocator>> =
    SpinMutex::new(Lazy::new(FrameAllocator::new));

struct FrameAllocator {
    top: PhysicalPageNum,
    bottom: PhysicalPageNum,
    // current should always point to the last frame that can be allocated
    current: PhysicalPageNum,
    recycled: Vec<PhysicalPageNum>,
}

impl FrameAllocator {
    pub fn new() -> Self {
        FrameAllocator {
            top: PhysicalPageNum::from_usize(usize::MAX),
            bottom: PhysicalPageNum::from_usize(usize::MAX),
            current: PhysicalPageNum::from_usize(usize::MAX),
            // Previous from_raw_parts(null, 0, 0) resulted this function highly optimized and inlined.
            // The compiler make this method inlined with no ret instruction. (I think was during monomorphization)
            // Instead, it generated an unimp instruction at the end of the function
            // which caused the kernel to panic when the function was called.
            // So we need to initialize the vector with some capacity to prevent this.
            recycled: Vec::new_in(alloc::alloc::Global), // lazy allocation, only when push is called
        }
    }

    pub fn init(&mut self, bottom: PhysicalPageNum, top: PhysicalPageNum) {
        self.bottom = bottom;
        self.top = top;
        self.current = bottom;
    }
}

impl IFrameAllocator for FrameAllocator {
    fn alloc_frame(&mut self) -> Option<TrackedFrame> {
        match self.recycled.pop() {
            Some(ppn) => Some(TrackedFrame::new(ppn)),
            None => match self.current {
                ppn if ppn < self.top => {
                    self.current = ppn + 1;
                    Some(TrackedFrame::new(ppn))
                }
                _ => None,
            },
        }
    }

    fn alloc_frames(&mut self, count: usize) -> Option<Vec<TrackedFrame>> {
        let mut frames = Vec::with_capacity(count);

        let avaliable = self.recycled.len() + (self.top - self.bottom).as_usize();

        match count {
            count if count <= avaliable => {
                for _ in 0..count {
                    match self.alloc_frame() {
                        Some(frame) => frames.push(frame),
                        None => break,
                    }
                }
                Some(frames)
            }
            // Prevent dealloc if we don't have enough frames
            _ => None,
        }
    }

    fn dealloc(&mut self, frame: &TrackedFrame) {
        // is valid frame
        debug_assert!(frame.ppn() >= self.bottom && frame.ppn() < self.top);
        // is allocated frame
        debug_assert!(
            self.recycled.iter().all(|ppn| *ppn != frame.ppn()) && self.current != frame.ppn()
        );

        let ppn = frame.ppn();

        debug_assert!(ppn < self.current);

        self.recycled.push(ppn);
        self.recycled.sort();

        // try gc self.current before push to recycled
        // Check if the recycled or ppn can be contiguous
        match self.recycled.last() {
            Some(last) if *last + 1 == self.current => {
                let mut new_current = self.current;

                loop {
                    match self.recycled.pop() {
                        Some(ppn) if ppn + 1 == new_current => {
                            new_current = ppn;
                        }
                        Some(ppn) => {
                            self.recycled.push(ppn);
                            break;
                        }
                        None => break,
                    }
                }

                self.current = new_current;
            }
            _ => (),
        }
    }

    fn alloc_contiguous(&mut self, count: usize) -> Option<TrackedFrameRange> {
        let avaliable = (self.top - self.current).as_usize();

        match count {
            count if count <= avaliable => {
                let start = self.current;
                self.current += count;

                Some(TrackedFrameRange { start, count })
            }
            // Prevent dealloc if we don't have enough frames
            _ => None,
        }
    }
}

pub fn alloc_frame() -> Option<TrackedFrame> {
    unsafe { FRAME_ALLOCATOR.lock().alloc_frame() }
}

// Allocates `count` frames and returns them as a vector
// No guarantee that the frames are contiguous
pub fn alloc_frames(count: usize) -> Option<Vec<TrackedFrame>> {
    unsafe { FRAME_ALLOCATOR.lock().alloc_frames(count) }
}

// Similar to alloc_frames, but guarantees that the frames are contiguous
pub fn alloc_contiguous(count: usize) -> Option<TrackedFrameRange> {
    unsafe { FRAME_ALLOCATOR.lock().alloc_contiguous(count) }
}

/// # Safety
/// This function is unsafe because we should we TrackedFrame or TrackedFrameRange to deallocate frames
/// But if you are using forget, you can use this function to deallocate frames
/// Still, you should not use this function unless you know what you are doing
pub unsafe fn dealloc_frame_unchecked(frame: PhysicalPageNum) {
    drop(TrackedFrame(frame))
}

fn dealloc_frame(frame: &TrackedFrame) {
    unsafe {
        FRAME_ALLOCATOR.lock().dealloc(frame);
    }
}

pub fn init_frame_allocator(memory_end: usize) {
    extern "C" {
        fn ekernel();
    }

    let bottom = ekernel as usize & constants::PHYS_ADDR_MASK;

    debug!(
        "Initializing frame allocator at {:#018x}..{:#018x}",
        bottom, memory_end
    );

    unsafe {
        FRAME_ALLOCATOR.lock().init(
            PhysicalPageNum::from_addr_ceil(PhysicalAddress::from_usize(bottom)),
            PhysicalPageNum::from_addr_floor(PhysicalAddress::from_usize(memory_end)),
        );
    }
}

// Returns in (avaliable, fragmented, total)
pub fn allocation_statistics() -> (usize, usize, usize) {
    let allocator = unsafe { FRAME_ALLOCATOR.lock() };

    (
        allocator.recycled.len() + (allocator.top - allocator.current).as_usize(),
        (allocator.current - allocator.bottom).as_usize(),
        (allocator.top - allocator.bottom).as_usize(),
    )
}
