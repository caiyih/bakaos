mod context;
mod registers;
mod serial;

use core::ffi::CStr;

// IMPORTANT: Must provide for every platform
pub(crate) use context::TaskTrapContext;

// IMPORTANT: Must provide for every platform
pub use serial::*;

// IMPORTANT: Must provide for every platform
// FIXME: Figure out the correct value for this
pub const PLATFORM_STRING: &CStr = c"Loongarch64";

pub const PHYS_ADDR_MASK: usize = 0x0000_7FFF_FFFF_FFFF; // keep to lower half
pub const VIRT_ADDR_OFFSET: usize = 0x9000_0000_0000_0000; // to higher half

pub use registers::*;

// IMPORTANT: Must provide for every platform
#[inline(always)]
pub const fn virt_to_phys(vaddr: usize) -> usize {
    vaddr & PHYS_ADDR_MASK
}

// IMPORTANT: Must provide for every platform
#[inline(always)]
pub const fn phys_to_virt(paddr: usize) -> usize {
    (paddr & PHYS_ADDR_MASK) | VIRT_ADDR_OFFSET
}

#[inline(always)]
pub fn current_processor_index() -> usize {
    r21()
}
