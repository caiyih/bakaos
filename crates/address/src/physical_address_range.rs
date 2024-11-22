use crate::*;

pub type PhysicalAddressRange = AddressRange<PhysicalAddress>;

impl_range_display!(PhysicalAddressRange);

impl PhysicalAddressRange {
    pub fn to_high_virtual(&self) -> VirtualAddressRange {
        VirtualAddressRange::from_start_end(self.start().to_high_virtual(), self.end().to_high_virtual())
    }
}
