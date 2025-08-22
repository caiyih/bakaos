#![no_std]
extern crate alloc;

pub mod memory;
pub mod memory_map;
pub mod page_table;

pub use memory::*;
pub use memory_map::*;
pub use page_table::{
    IWithPageGuardBuilder, MustHavePageGuard, PageGuardBuilder, PageTable,
    TemporaryModificationGuard, WithPageGuard,
};

pub fn init(kernel_table: PageTable) {
    page_table::init_kernel_page_table(kernel_table);
}
