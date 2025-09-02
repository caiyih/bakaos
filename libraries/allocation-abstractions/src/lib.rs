#![cfg_attr(not(feature = "std"), no_std)]

use alloc::vec::Vec;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod frame;

pub use frame::*;

pub trait IFrameAllocator {
    fn alloc_frame(&mut self) -> Option<FrameDesc>;
    // Allocates `count` frames and returns them as a vector, no guarantee that the frames are contiguous
    fn alloc_frames(&mut self, count: usize) -> Option<Vec<FrameDesc>>;
    // Allocates `count` frames and returns them as a range, guaranteeing that the frames are contiguous
    fn alloc_contiguous(&mut self, count: usize) -> Option<FrameRangeDesc>;

    fn dealloc(&mut self, frame: FrameDesc);

    fn dealloc_range(&mut self, range: FrameRangeDesc);
}
