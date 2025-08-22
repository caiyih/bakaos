#![no_std]

#[rustfmt::skip]
mod generated;

#[allow(unused_imports)]
pub use generated::*;

const _FORCE_REBUILD: &str = env!("FORCE_REBUILD_TS");

mod errno;
pub use errno::{ErrNo, SyscallError};

pub const PROCESSOR_COUNT: usize = 2;

pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_HEAP_SIZE: usize = 0x0200_0000;
pub const USER_STACK_SIZE: usize = 0x10_0000; // 1MB
