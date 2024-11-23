#![feature(allocator_api)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod memory;
pub mod page_table;

pub use memory::*;
pub use page_table::{
    IRawPageTable, IWithPageGuardBuilder, MustHavePageGuard, PageGuardBuilder, PageTable,
    PageTableEntry, PageTableEntryFlags, TemporaryModificationGuard, WithPageGuard,
};

pub fn init(kernel_table: PageTable) {
    page_table::init_kernel_page_table(kernel_table);
}
