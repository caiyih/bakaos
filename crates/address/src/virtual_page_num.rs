use abstractions::IUsizeAlias;

use crate::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualPageNum(pub usize);

impl_IPageNum!(VirtualPageNum);

// No need to test VirtualPageNum, as they share the same code as PhysicalPageNum
