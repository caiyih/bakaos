#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod auxv;
mod elf;
mod loader;
mod process;
mod shebang;
mod stack;

use alloc::sync::Arc;
use allocation_abstractions::IFrameAllocator;
use hermit_sync::SpinMutex;
pub use loader::*;
use mmu_abstractions::IMMU;
pub use process::*;

pub type RawMemorySpace = (
    Arc<SpinMutex<dyn IMMU>>,
    Arc<SpinMutex<dyn IFrameAllocator>>,
);
