use crate::*;

pub type VirtualPageNumRange = PageNumRange<VirtualPageNum>;

impl_range_display!(VirtualPageNumRange);

impl VirtualPageNumRange {
    pub fn identity_mapped(&self) -> PhysicalPageNumRange {
        PhysicalPageNumRange::from_start_end(
            self.start().identity_mapped(),
            self.end().identity_mapped(),
        )
    }
}
