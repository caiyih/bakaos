use alloc::vec::Vec;
use core::iter::Iterator;
use core::ops::Drop;
use core::usize;

use address::{IAddressBase, IPageNum, IPageNumBase, PhysicalAddress, PhysicalPageNum, PhysicalPageNumRange};
use hermit_sync::Lazy;
use log::debug;

pub struct TrackedFrame(PhysicalPageNum);

impl TrackedFrame {
    fn new(ppn: PhysicalPageNum) -> Self {
        TrackedFrame(ppn)
    }

    pub fn ppn(&self) -> PhysicalPageNum {
        self.0
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

static mut FRAME_ALLOCATOR: Lazy<FrameAllocator> = Lazy::new(FrameAllocator::new);

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
            recycled: unsafe { Vec::from_raw_parts(core::ptr::null_mut(), 0, 0) },
        }
    }

    pub fn init(&mut self, bottom: PhysicalPageNum, top: PhysicalPageNum) {
        self.bottom = bottom;
        self.top = top;
        self.current = bottom;
        self.recycled = Vec::new();
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

        // try gc self.current before push to recycled
        let mut current = self.current - 1;
        while current > ppn || self.recycled.iter().any(|ppn| *ppn == current - 1) {
            current -= 1;
        }

        let old_current = self.current;
        if old_current == current {
            self.recycled.push(ppn);
            self.recycled.sort(); // keep recycled sorted
        } else {
            self.current = current;
            let cutoff_at = self
                .recycled
                .iter()
                .enumerate()
                .find(|f| f.1 > &current)
                .map(|f| f.0);

            debug_assert!(cutoff_at.is_some());

            if let Some(cutoff_at) = cutoff_at {
                self.recycled.truncate(cutoff_at);
            }
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
    unsafe { FRAME_ALLOCATOR.alloc_frame() }
}

// Allocates `count` frames and returns them as a vector
// No guarantee that the frames are contiguous
pub fn alloc_frames(count: usize) -> Option<Vec<TrackedFrame>> {
    unsafe { FRAME_ALLOCATOR.alloc_frames(count) }
}

// Similar to alloc_frames, but guarantees that the frames are contiguous
pub fn alloc_contiguous(count: usize) -> Option<TrackedFrameRange> {
    unsafe { FRAME_ALLOCATOR.alloc_contiguous(count) }
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
        FRAME_ALLOCATOR.dealloc(frame);
    }
}

pub fn init_frame_allocator(memory_end: usize) {
    extern "C" {
        fn ekernel();
    }

    let bottom = ekernel as usize & constants::PHYS_ADDR_MASK;

    debug!(
        "Initializing frame allocator at {:#018x}..{:#018x}",
        bottom,
        memory_end
    );

    unsafe {
        FRAME_ALLOCATOR.init(
            PhysicalPageNum::from_addr_ceil(PhysicalAddress::from_usize(bottom)),
            PhysicalPageNum::from_addr_floor(PhysicalAddress::from_usize(memory_end)),
        );
    }
}
