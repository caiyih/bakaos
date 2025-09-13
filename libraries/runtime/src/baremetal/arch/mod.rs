#[cfg(target_arch = "riscv64")]
pub mod riscv64;

#[cfg(target_arch = "riscv64")]
pub use riscv64 as current;

use core::{alloc::Layout, ptr::NonNull};
use hermit_sync::SpinMutex;
