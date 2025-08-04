use core::fmt::Debug;

use abstractions::IUsizeAlias;
use address::PhysicalAddress;
use mmu_abstractions::GenericMappingFlags;

use crate::{
    pte::IGenericMappingFlags, IArchPageTableEntry, IArchPageTableEntryBase,
};

bitflags::bitflags! {
    /// Page-table entry flags.
    ///
    /// <https://loongson.github.io/LoongArch-Documentation/LoongArch-Vol1-EN.html#tlb-refill-exception-entry-low-order-bits>
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
    pub struct LA64PageTableEntryFlags: u64 {
        /// Whether the PTE is valid.
        const V = 1 << 0;
        /// Indicates the virtual page has been written since the last time the
        /// D bit was cleared.
        const D = 1 << 1;
        /// Privilege Level Low Bit
        const PLVL = 1 << 2;
        /// Privilege Level High Bit
        const PLVH = 1 << 3;
        /// Memory Access Type (MAT) of the page table entry.
        ///
        /// <https://loongson.github.io/LoongArch-Documentation/LoongArch-Vol1-EN.html#section-memory-access-types>
        ///
        /// 0 - SUC, 1 - CC, 2 - WUC
        ///
        /// 0 for strongly-ordered uncached,
        ///
        /// 1 for coherent cached,
        ///
        /// 2 for weakly-ordered uncached, and 3 for reserved.
        ///
        /// Memory Access Type Low Bit
        /// controls the type of access
        const MATL = 1 << 4;
        /// Memory Access Type High Bit
        const MATH = 1 << 5;
        /// Designates a global mapping OR Whether the page is huge page.
        const GH = 1 << 6;
        /// Whether the physical page is exist.
        const P = 1 << 7;
        /// Whether the page is writable.
        const W = 1 << 8;
        /// Designates a global mapping when using huge page.
        const G = 1 << 12;
        /// Whether the page is not readable.
        const NR = 1 << 61;
        /// Whether the page is not executable.
        const NX = 1 << 62;
        /// Whether the privilege Level is restricted. When RPLV is 0, the PTE
        /// can be accessed by any program with privilege Level highter than PLV.
        const RPLV = 1 << 63;
    }
}

impl const IGenericMappingFlags for GenericMappingFlags {
    type ArchMappingFlags = LA64PageTableEntryFlags;

    fn to_arch(self) -> LA64PageTableEntryFlags {
        let bits = self.bits();

        LA64PageTableEntryFlags::from_bits_truncate(
            (LA64PageTableEntryFlags::V.bits()
                | LA64PageTableEntryFlags::P.bits()
                | LA64PageTableEntryFlags::D.bits()
                | (((!bits & GenericMappingFlags::Readable.bits()) as u64) << 61)
                | (((bits & GenericMappingFlags::Writable.bits()) as u64) << (8 - 1))
                | (((!bits & GenericMappingFlags::Executable.bits()) as u64) << (62 - 2))
                | ((((bits & GenericMappingFlags::User.bits()) >> 3) as u64) * 0b1100) // PLVL and PLVH
                | (((!bits & GenericMappingFlags::Device.bits()) >> 5) as u64
                    * (((bits & GenericMappingFlags::Uncached.bits()) as u64 >> 1)
                        | ((!bits & GenericMappingFlags::Uncached.bits()) as u64 >> 2))))
                * (bits != 0) as u64,
        )
    }

    fn from_arch(f: LA64PageTableEntryFlags) -> Self {
        let bits = f.bits();

        Self::from_bits_truncate(
            (((!bits & LA64PageTableEntryFlags::NR.bits()) >> 61) // readable
                | ((bits & LA64PageTableEntryFlags::W.bits()) >> (8 - 1)) // writable
                | ((!bits & LA64PageTableEntryFlags::NX.bits()) >> (62 - 2)) // executable
                | (((bits & 0b1100 != 0) as u64) << 3) // user, has PLVL or PLVHs
                | ((((!bits) & LA64PageTableEntryFlags::MATL.bits()) >> 4)
                    * (((bits & LA64PageTableEntryFlags::MATH.bits()) << 1) // Uncached
                        | ((!bits) & LA64PageTableEntryFlags::MATH.bits())))) as usize // Device
                * (bits as usize & 0b1),
        )
    }
}

const PHYS_ADDR_MASK: u64 = 0x0000_ffff_ffff_f000; // bits 12..48

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LA64PageTableEntry(u64);

impl LA64PageTableEntry {
    const fn flags_internal(self) -> LA64PageTableEntryFlags {
        LA64PageTableEntryFlags::from_bits_truncate(self.bits())
    }
}

impl const IArchPageTableEntryBase for LA64PageTableEntry {
    type RawType = u64;

    fn from_bits(bits: Self::RawType) -> Self {
        LA64PageTableEntry(bits)
    }

    fn bits(&self) -> Self::RawType {
        self.0
    }

    fn empty() -> Self {
        Self::from_bits(0)
    }

    fn is_present(&self) -> bool {
        self.flags_internal().contains(LA64PageTableEntryFlags::P)
    }

    fn is_huge(&self) -> bool {
        self.flags_internal().contains(LA64PageTableEntryFlags::GH)
    }

    fn is_empty(&self) -> bool {
        self.bits() == 0
    }

    fn new_table(paddr: PhysicalAddress) -> Self {
        Self((paddr.as_usize() as u64) & PHYS_ADDR_MASK)
    }

    fn paddr(&self) -> PhysicalAddress {
        PhysicalAddress::from_usize((self.bits() & PHYS_ADDR_MASK) as usize)
    }

    fn flags(&self) -> GenericMappingFlags {
        GenericMappingFlags::from_arch(self.flags_internal())
    }

    fn new_page(paddr: address::PhysicalAddress, flags: GenericMappingFlags, huge: bool) -> Self {
        let mut flags = flags.to_arch();
        if huge {
            flags = flags.union(LA64PageTableEntryFlags::GH);
        }
        Self(flags.bits() | ((paddr.as_usize()) as u64 & PHYS_ADDR_MASK))
    }
}

impl IArchPageTableEntry for LA64PageTableEntry {
    fn set_paddr(&mut self, paddr: PhysicalAddress) {
        self.0 = (self.bits() & !PHYS_ADDR_MASK) | (paddr.as_usize() as u64 & PHYS_ADDR_MASK)
    }

    fn set_flags(&mut self, flags: GenericMappingFlags, huge: bool) {
        let mut flags = flags.to_arch();
        if huge {
            flags |= LA64PageTableEntryFlags::GH;
        }
        self.0 = (self.bits() & PHYS_ADDR_MASK) | flags.bits();
    }

    fn clear(&mut self) {
        self.0 = 0;
    }

    fn remove_flags(&mut self, flags: GenericMappingFlags) {
        let to_remove = flags.to_arch().bits() & 0b110000; // FIXME: we dont want to change remove MAT bits
        self.0 &= !to_remove;
    }

    fn add_flags(&mut self, flags: GenericMappingFlags) {
        let to_add = flags.to_arch().bits();
        self.0 |= to_add;
    }
}

impl Debug for LA64PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RV64PageTableEntry")
            .field("paddr", &self.paddr())
            .field("flags", &self.flags_internal())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_arch_empty() {
        let flags = GenericMappingFlags::empty();
        let arch_flags = flags.to_arch();
        assert!(arch_flags.is_empty(), "{:?}", arch_flags);
    }

    #[test]
    fn test_to_arch_readable() {
        let flags = GenericMappingFlags::Readable;
        let arch_flags = flags.to_arch();
        assert!(arch_flags.contains(LA64PageTableEntryFlags::V));
        assert!(arch_flags.contains(LA64PageTableEntryFlags::P));
        assert!(arch_flags.contains(LA64PageTableEntryFlags::D));
        assert!(!arch_flags.contains(LA64PageTableEntryFlags::NR));
        assert!(!arch_flags.contains(LA64PageTableEntryFlags::W));
        assert!(arch_flags.contains(LA64PageTableEntryFlags::NX));
    }

    #[test]
    fn test_to_arch_writable() {
        let flags = GenericMappingFlags::Writable;
        let arch_flags = flags.to_arch();
        assert!(arch_flags.contains(LA64PageTableEntryFlags::W));
    }

    #[test]
    fn test_to_arch_executable() {
        let flags = GenericMappingFlags::Executable;
        let arch_flags = flags.to_arch();
        assert!(!arch_flags.contains(LA64PageTableEntryFlags::NX));
    }

    #[test]
    fn test_to_arch_user() {
        let flags = GenericMappingFlags::User;
        let arch_flags = flags.to_arch();
        assert!(arch_flags.contains(LA64PageTableEntryFlags::PLVL | LA64PageTableEntryFlags::PLVH));
    }

    #[test]
    fn test_to_arch_device() {
        let flags = GenericMappingFlags::Device;
        let arch_flags = flags.to_arch();
        assert!(
            !arch_flags.intersects(LA64PageTableEntryFlags::MATL | LA64PageTableEntryFlags::MATH)
        );
    }

    #[test]
    fn test_to_arch_uncached() {
        let flags = GenericMappingFlags::Uncached;
        let arch_flags = flags.to_arch();
        assert!(arch_flags.contains(LA64PageTableEntryFlags::MATH));
        assert!(!arch_flags.contains(LA64PageTableEntryFlags::MATL));
    }

    #[test]
    fn test_to_arch_coherent_cached() {
        let flags = GenericMappingFlags::Readable;
        let arch_flags = flags.to_arch();
        assert!(arch_flags.contains(LA64PageTableEntryFlags::MATL));
        assert!(!arch_flags.contains(LA64PageTableEntryFlags::MATH));
    }

    #[test]
    fn test_to_arch_combination() {
        let flags = GenericMappingFlags::Readable
            | GenericMappingFlags::Writable
            | GenericMappingFlags::User
            | GenericMappingFlags::Uncached;
        let arch_flags = flags.to_arch();
        assert!(arch_flags.contains(
            LA64PageTableEntryFlags::V | LA64PageTableEntryFlags::P | LA64PageTableEntryFlags::D
        ));
        assert!(!arch_flags.contains(LA64PageTableEntryFlags::NR));
        assert!(arch_flags.contains(LA64PageTableEntryFlags::W));
        assert!(arch_flags.contains(LA64PageTableEntryFlags::PLVL | LA64PageTableEntryFlags::PLVH));
        assert!(arch_flags.contains(LA64PageTableEntryFlags::MATH));
    }

    #[test]
    fn test_from_arch_invalid() {
        let arch_flags = LA64PageTableEntryFlags::empty();
        let generic = GenericMappingFlags::from_arch(arch_flags);
        assert_eq!(generic, GenericMappingFlags::empty());
    }

    #[test]
    fn test_from_arch_device() {
        let arch_flags =
            LA64PageTableEntryFlags::V | LA64PageTableEntryFlags::P | LA64PageTableEntryFlags::D;
        let generic = GenericMappingFlags::from_arch(arch_flags);
        assert!(generic.contains(GenericMappingFlags::Device));
    }

    #[test]
    fn test_from_arch_uncached() {
        let arch_flags = LA64PageTableEntryFlags::V | LA64PageTableEntryFlags::MATH;
        let generic = GenericMappingFlags::from_arch(arch_flags);
        assert!(generic.contains(GenericMappingFlags::Uncached));
    }

    #[test]
    fn test_from_arch_coherent_cached() {
        let arch_flags = LA64PageTableEntryFlags::V | LA64PageTableEntryFlags::MATL;
        let generic = GenericMappingFlags::from_arch(arch_flags);
        assert!(!generic.contains(GenericMappingFlags::Device | GenericMappingFlags::Uncached));
    }

    #[test]
    fn test_from_arch_user() {
        let arch_flags = LA64PageTableEntryFlags::V
            | LA64PageTableEntryFlags::PLVL
            | LA64PageTableEntryFlags::PLVH;
        let generic = GenericMappingFlags::from_arch(arch_flags);
        assert!(generic.contains(GenericMappingFlags::User));
    }

    #[test]
    fn test_from_arch_readable() {
        let arch_flags = LA64PageTableEntryFlags::V;
        let generic = GenericMappingFlags::from_arch(arch_flags);
        assert!(generic.contains(GenericMappingFlags::Readable));
    }

    #[test]
    fn test_from_arch_writable() {
        let arch_flags = LA64PageTableEntryFlags::V | LA64PageTableEntryFlags::W;
        let generic = GenericMappingFlags::from_arch(arch_flags);
        assert!(generic.contains(GenericMappingFlags::Writable));
    }

    #[test]
    fn test_from_arch_not_executable() {
        let arch_flags = LA64PageTableEntryFlags::V | LA64PageTableEntryFlags::NX;
        let generic = GenericMappingFlags::from_arch(arch_flags);
        assert!(!generic.contains(GenericMappingFlags::Executable));
    }

    #[test]
    fn test_from_arch_complex() {
        let arch_flags = LA64PageTableEntryFlags::V
            | LA64PageTableEntryFlags::W
            | LA64PageTableEntryFlags::PLVL
            | LA64PageTableEntryFlags::PLVH
            | LA64PageTableEntryFlags::MATH;
        let generic = GenericMappingFlags::from_arch(arch_flags);
        assert!(generic.contains(GenericMappingFlags::Writable));
        assert!(generic.contains(GenericMappingFlags::User));
        assert!(generic.contains(GenericMappingFlags::Uncached));
        assert!(!generic.contains(GenericMappingFlags::Device));
    }

    #[test]
    fn mat_mapping() {
        let device = GenericMappingFlags::Device;
        assert!(!device
            .to_arch()
            .intersects(LA64PageTableEntryFlags::MATL | LA64PageTableEntryFlags::MATH));

        let uncached = GenericMappingFlags::Uncached;
        assert!(uncached.to_arch().contains(LA64PageTableEntryFlags::MATH));
    }

    #[test]
    fn privilege_level() {
        let user = GenericMappingFlags::User;
        let arch = user.to_arch();
        assert!(arch.contains(LA64PageTableEntryFlags::PLVL | LA64PageTableEntryFlags::PLVH));
    }
}
