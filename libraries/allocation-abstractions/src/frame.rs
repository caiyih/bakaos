use core::ops::{Deref, Drop, Range};

use address::PhysicalAddress;

#[derive(Debug)]
pub struct FrameDesc(pub PhysicalAddress);

impl FrameDesc {
    /// Create a new frame descriptor
    ///
    /// # Safety
    ///
    /// The caller must ensure that the frame is allocated.
    ///
    /// The caller is responsible for deallocating the frame.
    pub unsafe fn new(addr: PhysicalAddress) -> Self {
        FrameDesc(addr)
    }
}

impl Deref for FrameDesc {
    type Target = PhysicalAddress;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for FrameDesc {
    fn drop(&mut self) {
        panic!("You must manually deallocate frames")
    }
}

pub struct FrameRangeDesc {
    range: Range<PhysicalAddress>,
}

impl FrameRangeDesc {
    /// Create a new frame range descriptor
    ///
    /// # Safety
    ///
    /// The caller must ensure that the frames are allocated.
    ///
    /// The caller is responsible for deallocating the frames.
    pub unsafe fn new(start: PhysicalAddress, len: usize) -> Self {
        Self {
            range: start..start + len,
        }
    }
}

impl Deref for FrameRangeDesc {
    type Target = Range<PhysicalAddress>;

    fn deref(&self) -> &Self::Target {
        &self.range
    }
}

impl Drop for FrameRangeDesc {
    fn drop(&mut self) {
        panic!("You must manually deallocate frames")
    }
}
