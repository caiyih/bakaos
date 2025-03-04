#![feature(const_trait_impl)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

#[allow(unused_imports)]
pub use arch::*;
pub use flush_handle::*;
#[allow(unused_imports)]
pub use pt::*;
pub use pte::{
    GenericMappingFlags, IArchPageTableEntry, IArchPageTableEntryBase, IGenericMappingFlags,
};

mod arch;
mod flush_handle;
mod pt;
mod pte;

#[cfg(target_arch = "riscv64")]
pub type PageTable64Impl = PageTable64<SV39PageTableAttribute, RV64PageTableEntry>;

#[cfg(target_arch = "riscv64")]
pub type FlushHandleImpl = FlushHandle<SV39PageTableAttribute>;

/// The error type for page table operation failures.
#[derive(Debug, PartialEq, Eq)]
pub enum PagingError {
    /// The address is not aligned to the page size.
    NotAligned,
    /// The mapping is not present.
    NotMapped,
    /// The mapping is already present.
    AlreadyMapped,
    /// The page table entry represents a huge page, but the target physical
    /// frame is 4K in size.
    MappedToHugePage,
}

/// The page sizes supported by the hardware page table.
#[repr(usize)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PageSize {
    /// Size of 4 kilobytes (2<sup>12</sup> bytes).
    _4K = 0x1000,
    /// Size of 2 megabytes (2<sup>21</sup> bytes).
    _2M = 0x20_0000,
    /// Size of 1 gigabytes (2<sup>30</sup> bytes).
    _1G = 0x4000_0000,
}

impl PageSize {
    pub const fn alignment(&self) -> usize {
        match self {
            PageSize::_4K => 0x1000,
            PageSize::_2M => 0x20_0000,
            PageSize::_1G => 0x4000_0000,
        }
    }
}

pub type PagingResult<TValue> = Result<TValue, PagingError>;
