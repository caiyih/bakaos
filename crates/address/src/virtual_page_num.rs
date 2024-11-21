use abstractions::IUsizeAlias;

use crate::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualPageNum(pub usize);

impl_IPageNum!(VirtualPageNum);

const VIRT_PAGE_NUM_WIDTH: usize = 9;
const VIRT_PAGE_NUM_MASK: usize = (1 << VIRT_PAGE_NUM_WIDTH) - 1;

impl VirtualPageNum {
    // Construct 3-level page table indices from a virtual page number
    // The indices are in the order of [level 2(root), level 1, level 0(leaf)]
    pub fn page_table_indices(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & VIRT_PAGE_NUM_MASK;
            vpn >>= VIRT_PAGE_NUM_WIDTH;
        }
        idx
    }
}

// No need to test VirtualPageNum, as they share the same code as PhysicalPageNum
