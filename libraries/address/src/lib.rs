#![feature(cfg_accessible)]
#![feature(const_trait_impl)]
#![feature(debug_closure_helpers)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod address;
mod address_range;
mod page_num;
mod page_num_range;
mod physical_address;
mod physical_address_range;
mod physical_page_num;
mod physical_page_num_range;
mod virtual_address;
mod virtual_address_range;
mod virtual_page_num;
mod virtual_page_num_range;

pub use address::*;
pub use address_range::*;
pub use page_num::*;
pub use page_num_range::*;
pub use physical_address::*;
pub use physical_address_range::*;
pub use physical_page_num::*;
pub use physical_page_num_range::*;
pub use virtual_address::*;
pub use virtual_address_range::*;
pub use virtual_page_num::*;
pub use virtual_page_num_range::*;

pub const PAGE_SIZE_BITS: usize = 0xc;

#[const_trait]
pub trait IConvertablePhysicalAddress {
    fn to_high_virtual(&self) -> VirtualAddress;

    fn as_virtual(addr: usize) -> usize;

    fn is_valid_pa(addr: usize) -> bool;
}

#[const_trait]
pub trait IConvertableVirtualAddress {
    fn to_low_physical(&self) -> PhysicalAddress;

    fn as_physical(addr: usize) -> usize;

    fn is_valid_va(addr: usize) -> bool;
}

#[cfg_accessible(::platform_specific::phys_to_virt)]
#[cfg_accessible(::platform_specific::virt_to_phys)]
impl const IConvertableVirtualAddress for VirtualAddress {
    #[inline(always)]
    fn to_low_physical(&self) -> PhysicalAddress {
        use abstractions::IUsizeAlias;
        PhysicalAddress::from_usize(Self::as_physical(self.as_usize()))
    }

    #[inline(always)]
    fn as_physical(addr: usize) -> usize {
        ::platform_specific::virt_to_phys(addr)
    }

    #[inline(always)]
    fn is_valid_va(addr: usize) -> bool {
        addr == ::platform_specific::phys_to_virt(addr)
    }
}

#[cfg_accessible(::platform_specific::virt_to_phys)]
#[cfg_accessible(::platform_specific::phys_to_virt)]
impl const IConvertablePhysicalAddress for PhysicalAddress {
    #[inline(always)]
    fn to_high_virtual(&self) -> VirtualAddress {
        use abstractions::IUsizeAlias;
        VirtualAddress::from_usize(Self::as_virtual(self.as_usize()))
    }

    #[inline(always)]
    fn as_virtual(addr: usize) -> usize {
        ::platform_specific::phys_to_virt(addr)
    }

    #[inline(always)]
    fn is_valid_pa(addr: usize) -> bool {
        addr == ::platform_specific::virt_to_phys(addr)
    }
}
