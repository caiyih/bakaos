#![feature(impl_trait_in_assoc_type)]
#![feature(associated_type_defaults)]
#![cfg_attr(not(feature = "std"), no_std)]

use alloc::sync::Arc;
use allocation_abstractions::IFrameAllocator;
use downcast_rs::{impl_downcast, Downcast};
use filesystem_abstractions::DirectoryTreeNode;
use hermit_sync::SpinMutex;
use mmu_abstractions::IMMU;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub trait IKernel: Downcast {
    fn serial(&self) -> Arc<dyn IKernelSerial>;

    fn fs(&self) -> Arc<SpinMutex<Arc<DirectoryTreeNode>>>;

    fn allocator(&self) -> Arc<SpinMutex<dyn IFrameAllocator>>;

    fn activate_mmu(&self, pt: &dyn IMMU);
}

impl_downcast!(IKernel);

pub trait IKernelSerial: Downcast {
    fn send(&self, byte: u8) -> Result<(), &'static str>;

    fn recv(&self) -> Option<u8>;
}

impl_downcast!(IKernelSerial);

pub trait ISyscallContext {
    fn fs(&self) -> Arc<DirectoryTreeNode>;

    fn kernel(&self) -> &dyn IKernel;
}
