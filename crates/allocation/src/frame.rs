use core::usize;

use address::{IPageNumBase, PhysicalPageNum};
use hermit_sync::Lazy;

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

trait IFrameAllocator {
    fn alloc_frame(&mut self) -> Option<TrackedFrame>;
    fn alloc_frames(&mut self, count: usize) -> Option<Vec<TrackedFrame>>;
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
    current: PhysicalPageNum,
    recycled: Vec<PhysicalPageNum>,
}

impl FrameAllocator {
    pub fn new() -> Self {
        FrameAllocator {
            top: PhysicalPageNum::from_usize(usize::MAX),
            bottom: PhysicalPageNum::from_usize(usize::MAX),
            current: PhysicalPageNum::from_usize(usize::MAX),
            recycled: unsafe { Vec::from_raw_parts(std::ptr::null_mut(), 0, 0) },
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

        match ppn.cmp(&self.current) {
            std::cmp::Ordering::Equal => unreachable!("Should panic at the debug build"),
            std::cmp::Ordering::Greater => self.recycled.push(ppn),
            std::cmp::Ordering::Less => {
                let previous = self.current;
                self.current = ppn;
                self.recycled.push(previous);
            }
        }
    }
}

pub fn alloc_frame() -> Option<TrackedFrame> {
    unsafe { FRAME_ALLOCATOR.alloc_frame() }
}

pub fn alloc_frames(count: usize) -> Option<Vec<TrackedFrame>> {
    unsafe { FRAME_ALLOCATOR.alloc_frames(count) }
}

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

    unsafe {
        FRAME_ALLOCATOR.init(
            PhysicalPageNum::from_usize(ekernel as usize),
            PhysicalPageNum::from_usize(memory_end),
        );
    }
}
