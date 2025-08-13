use address::PhysicalAddress;
use allocation_abstractions::IFrameAllocator;

pub mod contiguous;
pub mod segment;

pub trait ITestFrameAllocator: IFrameAllocator {
    fn check_paddr(&self, paddr: PhysicalAddress, len: usize) -> bool;

    fn linear_map(&self, paddr: PhysicalAddress) -> Option<*mut u8>;
}
