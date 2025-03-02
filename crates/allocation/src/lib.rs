#![feature(cfg_accessible)]
#![feature(allocator_api)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod frame;

pub use frame::{
    alloc_contiguous, alloc_frame, alloc_frames, allocation_statistics, dealloc_frame_unchecked,
    TrackedFrame, TrackedFrameRange,
};

pub fn init(bottom: usize, memory_end: usize) {
    frame::init_frame_allocator(bottom, memory_end);
}
