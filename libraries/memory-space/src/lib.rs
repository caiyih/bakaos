#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod mapping;
mod memory;

pub use mapping::*;
pub use memory::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapType {
    Identity,
    Framed,
    Direct,
    Linear,
}

/// Layout of a typical user memory space
/// +------------------+ <- MEMORY_END
/// |    Unallocated   |
/// |      Frames      |
/// +------------------+ <- += 0x0008_0000 (8MB)
/// |   kernel heap    |   
/// +------------------+ <- ekernel
/// | High half kernel |
/// +------------------+ <- 0xffff_ffc0_8020_0000
/// |    Mapped SBI    |
/// +------------------+ <- 0xffff_ffc0_4000_0000
/// |    Mapped MMIO   |
/// +------------------+ <- 0xffff_ffc0_0000_0000
/// |                  |
/// |                  |
/// |                  |
/// |       void       |
/// |                  |
/// |                  |
/// |                  |
/// +------------------+ <- += 0x0000
/// |       Brk        |       empty at the beginning, dynamically grows or shrinks
/// +------------------+ <- += 0x1000
/// | Stack Guard Top  |
/// +------------------+ <- += USER_STACK_SIZE
/// |                  |
/// |    User stack    |
/// |                  |
/// +------------------+ <- += 0x1000
/// | Stack Guard Base |
/// +------------------+ <- 0x0000_0000_0060_0000
/// |                  |
/// |        ELF       |
/// |                  |
/// +------------------+ <- 0x0000_8000_0000_1000
/// |                  |
/// +------------------+ <- 0x0000_0000_0000_0000
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaType {
    UserElf,
    SignalTrampoline,
    UserStackGuardBase,
    UserStack,
    UserStackGuardTop,
    UserBrk,
    VMA,
    Kernel,
}
