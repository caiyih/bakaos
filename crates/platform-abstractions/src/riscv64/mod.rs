mod boot;
pub use boot::_start;

pub const VIRT_ADDR_OFFSET: usize = 0xffff_ffc0_0000_0000;
pub const PHYS_ADDR_MASK: usize = 0x0000_003f_ffff_ffff;
