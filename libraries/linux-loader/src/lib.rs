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

pub use loader::*;
pub use process::*;

pub type RawMemorySpace = (
    alloc::sync::Arc<hermit_sync::SpinMutex<dyn mmu_abstractions::IMMU>>,
    alloc::sync::Arc<hermit_sync::SpinMutex<dyn allocation_abstractions::IFrameAllocator>>,
);
