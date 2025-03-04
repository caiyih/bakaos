use ::core::fmt::Debug;

use abstractions::IUsizeAlias;
use address::PhysicalAddress;

use crate::pte::{
    GenericMappingFlags, IArchPageTableEntry, IArchPageTableEntryBase, IGenericMappingFlags,
};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
    pub struct RV64PageTableEntryFlags : usize {
        const Valid = 1 << 0;
        const Readable = 1 << 1;
        const Writable = 1 << 2;
        const Executable = 1 << 3;
        const User = 1 << 4;
        const Global = 1 << 5;
        const Accessed = 1 << 6;
        const Dirty = 1 << 7;
        const _Reserved8 = 1 << 8;
    }
}

const RV64_USER_MASK: usize = RV64PageTableEntryFlags::User.bits();
const RV64_USER_OFFSET: usize = 4;
const GENERIC_USER_MASK: usize = GenericMappingFlags::User.bits();
const GENERIC_USER_OFFSET: usize = 3;

const RV64_READABLE_MASK: usize = RV64PageTableEntryFlags::Readable.bits();
const RV64_READABLE_OFFSET: usize = 1;
const GENERIC_READABLE_MASK: usize = GenericMappingFlags::Readable.bits();
const GENERIC_READABLE_OFFSET: usize = 0;

const RV64_WRITABLE_MASK: usize = RV64PageTableEntryFlags::Writable.bits();
const RV64_WRITABLE_OFFSET: usize = 2;
const GENERIC_WRITABLE_MASK: usize = GenericMappingFlags::Writable.bits();
const GENERIC_WRITABLE_OFFSET: usize = 1;

const RV64_EXECUTABLE_MASK: usize = RV64PageTableEntryFlags::Executable.bits();
const RV64_EXECUTABLE_OFFSET: usize = 3;
const GENERIC_EXECUTABLE_MASK: usize = GenericMappingFlags::Executable.bits();
const GENERIC_EXECUTABLE_OFFSET: usize = 2;

impl const IGenericMappingFlags for GenericMappingFlags {
    type ArchMappingFlags = RV64PageTableEntryFlags;

    #[inline(always)]
    fn to_arch(self) -> Self::ArchMappingFlags {
        let bits = self.bits();

        RV64PageTableEntryFlags::empty()
            .union(RV64PageTableEntryFlags::from_bits_retain(
                (bits & GENERIC_USER_MASK) >> GENERIC_USER_OFFSET << RV64_USER_OFFSET,
            ))
            .union(RV64PageTableEntryFlags::from_bits_retain(
                (bits & GENERIC_READABLE_MASK) >> GENERIC_READABLE_OFFSET << RV64_READABLE_OFFSET,
            ))
            .union(RV64PageTableEntryFlags::from_bits_retain(
                (bits & GENERIC_WRITABLE_MASK) >> GENERIC_WRITABLE_OFFSET << RV64_WRITABLE_OFFSET,
            ))
            .union(RV64PageTableEntryFlags::from_bits_retain(
                (bits & GENERIC_EXECUTABLE_MASK) >> GENERIC_EXECUTABLE_OFFSET
                    << RV64_EXECUTABLE_OFFSET,
            ))
    }

    #[inline(always)]
    fn from_arch(flags: Self::ArchMappingFlags) -> Self {
        let bits = flags.bits();

        GenericMappingFlags::Kernel // The kernel should be able to access the whole user space under RISC-V
            .union(GenericMappingFlags::from_bits_retain(
                ((bits & RV64_USER_MASK) >> RV64_USER_OFFSET) << GENERIC_USER_OFFSET,
            ))
            .union(GenericMappingFlags::from_bits_retain(
                ((bits & RV64_READABLE_MASK) >> RV64_READABLE_OFFSET) << GENERIC_READABLE_OFFSET,
            ))
            .union(GenericMappingFlags::from_bits_retain(
                ((bits & RV64_WRITABLE_MASK) >> RV64_WRITABLE_OFFSET) << GENERIC_WRITABLE_OFFSET,
            ))
            .union(GenericMappingFlags::from_bits_retain(
                ((bits & RV64_EXECUTABLE_MASK) >> RV64_EXECUTABLE_OFFSET)
                    << GENERIC_EXECUTABLE_OFFSET,
            ))
    }
}

const PTE_FLAGS_MASK: u64 = 0x1FF;
const PTE_PHYS_MASK: u64 = (1 << 54) - (1 << 10); // bits 10..54

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RV64PageTableEntry(u64);

impl RV64PageTableEntry {
    #[inline(always)]
    const fn flags_internal(&self) -> RV64PageTableEntryFlags {
        RV64PageTableEntryFlags::from_bits_truncate((self.bits() & PTE_FLAGS_MASK) as usize)
    }
}

impl const IArchPageTableEntryBase for RV64PageTableEntry {
    type RawType = u64;

    #[inline(always)]
    fn from_bits(bits: Self::RawType) -> Self {
        RV64PageTableEntry(bits)
    }

    #[inline(always)]
    fn bits(&self) -> Self::RawType {
        self.0
    }

    #[inline(always)]
    fn empty() -> Self {
        RV64PageTableEntry(0)
    }

    #[inline(always)]
    fn is_present(&self) -> bool {
        RV64PageTableEntryFlags::from_bits_truncate(self.bits() as usize)
            .contains(RV64PageTableEntryFlags::Valid)
    }

    #[inline(always)]
    fn is_huge(&self) -> bool {
        // workaround to determine if it's a huge page
        RV64PageTableEntryFlags::from_bits_truncate(self.0 as usize).intersects(
            RV64PageTableEntryFlags::union(
                RV64PageTableEntryFlags::Readable,
                RV64PageTableEntryFlags::Executable,
            ),
        )
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.bits() == 0
    }

    #[inline(always)]
    fn new_table(paddr: PhysicalAddress) -> Self {
        const FLAGS: RV64PageTableEntryFlags = RV64PageTableEntryFlags::Valid;
        Self::from_bits(((paddr.as_usize() >> 2) as u64 & PTE_PHYS_MASK) | FLAGS.bits() as u64)
    }

    #[inline(always)]
    fn paddr(&self) -> PhysicalAddress {
        PhysicalAddress::from_usize(((self.bits() & PTE_PHYS_MASK) << 2) as usize)
    }

    #[inline(always)]
    fn flags(&self) -> GenericMappingFlags {
        GenericMappingFlags::from_arch(self.flags_internal())
    }

    #[inline(always)]
    fn new_page(paddr: PhysicalAddress, flags: GenericMappingFlags, _huge: bool) -> Self {
        let flags = flags
            .to_arch()
            .union(RV64PageTableEntryFlags::Accessed)
            .union(RV64PageTableEntryFlags::Dirty);

        Self(flags.bits() as u64 | ((paddr.as_usize() >> 2) as u64 & PTE_PHYS_MASK))
    }
}

impl IArchPageTableEntry for RV64PageTableEntry {
    fn set_paddr(&mut self, paddr: PhysicalAddress) {
        self.0 = (self.0 & !(PTE_PHYS_MASK)) // keep flags
            | ((paddr.as_usize() as u64 >> 2) & PTE_PHYS_MASK); // new paddr
    }

    fn set_flags(&mut self, flags: GenericMappingFlags, _huge: bool) {
        let flags =
            flags.to_arch() | RV64PageTableEntryFlags::Accessed | RV64PageTableEntryFlags::Dirty;
        self.0 = (self.0 & PTE_PHYS_MASK) | flags.bits() as u64;
    }

    fn clear(&mut self) {
        self.0 = 0;
    }

    fn remove_flags(&mut self, flags: GenericMappingFlags) {
        let to_remove = !(flags.to_arch().bits() as u64);
        self.0 &= to_remove;
    }

    fn add_flags(&mut self, flags: GenericMappingFlags) {
        let to_add = flags.to_arch().bits() as u64;
        self.0 |= to_add;
    }
}

impl Debug for RV64PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RV64PageTableEntry")
            .field("paddr", &self.paddr())
            .field("flags", &self.flags_internal())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;
    use crate::pte::GenericMappingFlags;

    #[test]
    fn test_flag_conversions() {
        let gm_flags = GenericMappingFlags::Readable;
        let rv_flags = gm_flags.to_arch();
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Readable));
        assert_eq!(
            GenericMappingFlags::from_arch(rv_flags),
            gm_flags | GenericMappingFlags::Kernel
        );

        let gm_flags = GenericMappingFlags::Writable;
        let rv_flags = gm_flags.to_arch();
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Writable));
        assert_eq!(
            GenericMappingFlags::from_arch(rv_flags),
            gm_flags | GenericMappingFlags::Kernel
        );

        let gm_flags = GenericMappingFlags::Executable;
        let rv_flags = gm_flags.to_arch();
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Executable));
        assert_eq!(
            GenericMappingFlags::from_arch(rv_flags),
            gm_flags | GenericMappingFlags::Kernel
        );

        let gm_flags = GenericMappingFlags::User;
        let rv_flags = gm_flags.to_arch();
        assert!(rv_flags.contains(RV64PageTableEntryFlags::User));
        assert_eq!(
            GenericMappingFlags::from_arch(rv_flags),
            gm_flags | GenericMappingFlags::Kernel
        );

        let gm_flags = GenericMappingFlags::Readable
            | GenericMappingFlags::Writable
            | GenericMappingFlags::Executable
            | GenericMappingFlags::User;
        let rv_flags = gm_flags.to_arch();
        assert!(rv_flags.contains(
            RV64PageTableEntryFlags::Readable
                | RV64PageTableEntryFlags::Writable
                | RV64PageTableEntryFlags::Executable
                | RV64PageTableEntryFlags::User
        ));
        assert_eq!(
            GenericMappingFlags::from_arch(rv_flags),
            gm_flags | GenericMappingFlags::Kernel
        );

        let rv_flags = RV64PageTableEntryFlags::Valid | RV64PageTableEntryFlags::User;
        let gm_flags = GenericMappingFlags::from_arch(rv_flags);
        assert!(gm_flags.contains(GenericMappingFlags::User));
        assert!(gm_flags.contains(GenericMappingFlags::Kernel));

        let rv_flags = RV64PageTableEntryFlags::Valid;
        let gm_flags = GenericMappingFlags::from_arch(rv_flags);
        assert!(!gm_flags.contains(GenericMappingFlags::User));
        assert!(gm_flags.contains(GenericMappingFlags::Kernel));
    }

    #[test]
    fn test_pte_construction() {
        let paddr = PhysicalAddress::from_usize(0x4000);
        let flags = GenericMappingFlags::Readable | GenericMappingFlags::Writable;
        let pte = RV64PageTableEntry::new_page(paddr, flags, false);

        assert_eq!(pte.paddr(), paddr);

        let rv_flags = pte.flags_internal();
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Readable));
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Writable));
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Accessed));
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Dirty));
        assert!(!rv_flags.contains(RV64PageTableEntryFlags::Executable));
        assert!(!rv_flags.contains(RV64PageTableEntryFlags::User));

        let table_pte = RV64PageTableEntry::new_table(paddr);
        assert_eq!(table_pte.paddr(), paddr);
        assert!(table_pte
            .flags_internal()
            .contains(RV64PageTableEntryFlags::Valid));
        assert!(!table_pte
            .flags_internal()
            .contains(RV64PageTableEntryFlags::Accessed));
    }

    #[test]
    fn test_set_paddr() {
        let mut pte = RV64PageTableEntry::new_page(
            PhysicalAddress::from_usize(0x1000),
            GenericMappingFlags::empty(),
            false,
        );
        let new_paddr = PhysicalAddress::from_usize(0x2000);
        pte.set_paddr(new_paddr);
        assert_eq!(pte.paddr(), new_paddr);

        let original_flags = pte.flags_internal();
        pte.set_paddr(PhysicalAddress::from_usize(0x3000));
        assert_eq!(pte.flags_internal(), original_flags);
    }

    #[test]
    fn test_set_flags() {
        let mut pte = RV64PageTableEntry::new_page(
            PhysicalAddress::from_usize(0x1000),
            GenericMappingFlags::Readable,
            false,
        );

        pte.set_flags(
            GenericMappingFlags::Executable | GenericMappingFlags::User,
            false,
        );
        let rv_flags = pte.flags_internal();
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Executable));
        assert!(rv_flags.contains(RV64PageTableEntryFlags::User));

        assert!(rv_flags.contains(RV64PageTableEntryFlags::Accessed));
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Dirty));

        pte.set_flags(GenericMappingFlags::empty(), false);
        let rv_flags = pte.flags_internal();
        assert!(!rv_flags.contains(RV64PageTableEntryFlags::Readable));
        assert!(!rv_flags.contains(RV64PageTableEntryFlags::Writable));
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Accessed));
        assert!(rv_flags.contains(RV64PageTableEntryFlags::Dirty));
    }

    #[test]
    fn test_add_remove_flags() {
        let mut pte = RV64PageTableEntry::new_page(
            PhysicalAddress::from_usize(0x1000),
            GenericMappingFlags::Readable,
            false,
        );

        pte.add_flags(GenericMappingFlags::Writable);
        assert!(pte
            .flags_internal()
            .contains(RV64PageTableEntryFlags::Writable));

        pte.remove_flags(GenericMappingFlags::Readable);
        assert!(!pte
            .flags_internal()
            .contains(RV64PageTableEntryFlags::Readable));
        assert!(pte
            .flags_internal()
            .contains(RV64PageTableEntryFlags::Writable));

        assert!(pte
            .flags_internal()
            .contains(RV64PageTableEntryFlags::Accessed));
        assert!(pte
            .flags_internal()
            .contains(RV64PageTableEntryFlags::Dirty));
    }

    #[test]
    fn test_clear_pte() {
        let mut pte = RV64PageTableEntry::new_page(
            PhysicalAddress::from_usize(0x1000),
            GenericMappingFlags::Readable,
            false,
        );
        pte.clear();
        assert_eq!(pte.bits(), 0);
    }

    #[test]
    fn test_debug_output() {
        let pte = RV64PageTableEntry::new_page(
            PhysicalAddress::from_usize(0x1000),
            GenericMappingFlags::Executable,
            false,
        );
        let debug_str = format!("{:?}", pte);
        assert!(debug_str.contains("paddr: PhysicalAddress(0x1000)"));
        assert!(debug_str.contains("flags"));
        assert!(debug_str.contains("Executable"));
        assert!(debug_str.contains("Accessed"));
        assert!(debug_str.contains("Dirty"));
    }
}
