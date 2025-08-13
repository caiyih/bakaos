use crate::*;

pub type PhysicalPageNumRange = PageNumRange<PhysicalPageNum>;

impl_range_display!(PhysicalPageNumRange);
