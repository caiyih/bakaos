use crate::*;

pub type PhysicalPageNumRange = PageNumRange<PhysicalPageNum>;

impl_range_display!(PhysicalPageNumRange);

impl PhysicalPageNumRange {
    pub fn identity_mapped(&self) -> VirtualPageNumRange {
        VirtualPageNumRange::from_start_end(
            self.start().identity_mapped(),
            self.end().identity_mapped(),
        )
    }
}
