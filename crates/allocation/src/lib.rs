#![feature(allocator_api)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod frame;

pub use frame::{
    alloc_contiguous, alloc_frame, alloc_frames, dealloc_frame_unchecked, TrackedFrame,
    TrackedFrameRange,
};

pub fn init(memory_end: usize) {
    frame::init_frame_allocator(memory_end);
}
