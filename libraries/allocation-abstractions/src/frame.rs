use core::ops::{Deref, Drop, Range};

use address::PhysicalAddress;

#[derive(Debug)]
pub struct FrameDesc(pub PhysicalAddress);

impl FrameDesc {
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
