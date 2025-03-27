#![no_std]

mod build_info;
pub use build_info::*;

mod errno;
pub use errno::{ErrNo, SyscallError};

pub const PROCESSOR_COUNT: usize = 2;

pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_HEAP_SIZE: usize = 0x0100_0000;
pub const USER_STACK_SIZE: usize = 0x10_0000; // 1MB
