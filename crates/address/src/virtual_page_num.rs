use crate::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualPageNum(usize);

impl_IPageNum!(VirtualPageNum);

impl VirtualPageNum {
    pub fn identity_mapped(self) -> PhysicalPageNum {
        PhysicalPageNum::from_usize(self.as_usize())
    }
}

// No need to test VirtualPageNum, as they share the same code as PhysicalPageNum
