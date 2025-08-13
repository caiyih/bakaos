use crate::*;

pub type PhysicalAddressRange = AddressRange<PhysicalAddress>;

impl_range_display!(PhysicalAddressRange);
