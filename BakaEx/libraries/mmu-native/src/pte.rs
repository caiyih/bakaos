use address::PhysicalAddress;
use alloc::fmt::Debug;
use mmu_abstractions::GenericMappingFlags;

#[allow(unused)]
#[const_trait]
pub(crate) trait IGenericMappingFlags: Clone + Copy {
    type ArchMappingFlags;

    fn to_arch(self) -> Self::ArchMappingFlags;

    fn from_arch(flags: Self::ArchMappingFlags) -> Self;
}

#[const_trait]
pub trait IArchPageTableEntryBase:
    Debug + Clone + Copy + Sync + Send + Sized + Eq + PartialEq
{
    type RawType;

    fn from_bits(bits: Self::RawType) -> Self;
    fn bits(&self) -> Self::RawType;

    fn empty() -> Self;

    fn is_present(&self) -> bool;
    fn is_huge(&self) -> bool;

    fn is_empty(&self) -> bool;

    fn new_table(paddr: PhysicalAddress) -> Self;
    fn paddr(&self) -> PhysicalAddress;
    fn flags(&self) -> GenericMappingFlags;

    fn new_page(paddr: PhysicalAddress, flags: GenericMappingFlags, huge: bool) -> Self;
}

pub trait IArchPageTableEntry: const IArchPageTableEntryBase {
    fn set_paddr(&mut self, paddr: PhysicalAddress);
    fn set_flags(&mut self, flags: GenericMappingFlags, huge: bool);
    fn clear(&mut self);

    fn remove_flags(&mut self, flags: GenericMappingFlags);
    fn add_flags(&mut self, flags: GenericMappingFlags);
}
