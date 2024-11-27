#![no_std]

mod build_info;
pub use build_info::*;

pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_HEAP_SIZE: usize = 0x0080_0000;
pub const VIRT_ADDR_OFFSET: usize = 0xffff_ffc0_0000_0000;
pub const PHYS_ADDR_MASK: usize = 0x0000_003f_ffff_ffff;
pub const USER_STACK_SIZE: usize = 0x10_0000; // 1MB
