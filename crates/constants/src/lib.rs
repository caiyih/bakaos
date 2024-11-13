#![no_std]

pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_HEAP_SIZE: usize = 0x0080_0000;
pub const VIRT_ADDR_OFFSET: usize = 0xffff_ffc0_0000_0000;
