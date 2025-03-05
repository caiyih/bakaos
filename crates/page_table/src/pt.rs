use core::marker::PhantomData;

use crate::IArchPageTableEntry;
use address::{PhysicalAddress, VirtualAddress};
use alloc::{vec, vec::Vec};

#[cfg(target_os = "none")]
use crate::{FlushHandle, GenericMappingFlags, PageSize, PagingError, PagingResult};

#[cfg(target_os = "none")]
use address::{IAddressBase, IAlignableAddress, IConvertablePhysicalAddress, IToPageNum};

#[cfg(target_os = "none")]
use abstractions::IUsizeAlias;

pub trait IPageTableArchAttribute {
    const LEVELS: usize;
    const PA_MAX_BITS: usize;
    const VA_MAX_BITS: usize;
    const PA_MAX_ADDR: usize = (1 << Self::PA_MAX_BITS) - 1;

    // Pass Null address to flush all tlb
    fn flush_tlb(vaddr: VirtualAddress);

    fn is_higher_half_activated(paddr: PhysicalAddress) -> bool;

    fn is_lower_half_activated(paddr: PhysicalAddress) -> bool;

    fn activate(paddr: PhysicalAddress, lazy_flush: bool);

    fn activated_table() -> PhysicalAddress;
}

pub struct PageTable64<Arch, PTE>
where
    Arch: IPageTableArchAttribute,
    PTE: IArchPageTableEntry,
{
    root: PhysicalAddress,
    frames: Vec<PhysicalAddress>,
    _marker: PhantomData<(Arch, PTE)>,
}

impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTable64<Arch, PTE> {
    pub const fn from_borrowed(root: PhysicalAddress) -> Self {
        Self {
            root,
            frames: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn new(root: PhysicalAddress, owned: bool) -> Self {
        match owned {
            false => Self::from_borrowed(root),
            true => Self {
                root,
                frames: vec![root],
                _marker: PhantomData,
            },
        }
    }

    pub const fn root(&self) -> PhysicalAddress {
        self.root
    }

    pub fn is_owned(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            if self.frames.is_empty() {
                return false;
            }

            debug_assert_eq!(self.frames.first().cloned(), Some(self.root));

            true
        }

        #[cfg(not(debug_assertions))]
        {
            !self.frames.is_empty()
        }
    }

    pub fn activated_table() -> PhysicalAddress {
        Arch::activated_table()
    }

    pub fn flush_tlb(vaddr: VirtualAddress) {
        Arch::flush_tlb(vaddr);
    }
}

impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTable64<Arch, PTE> {
    #[inline(always)]
    pub fn is_higher_activated(&self) -> bool {
        Arch::is_higher_half_activated(self.root())
    }

    #[inline(always)]
    pub fn is_lower_activated(&self) -> bool {
        Arch::is_lower_half_activated(self.root())
    }

    #[allow(unused_mut)]
    pub fn activate(&self, mut lazy_flush: bool) {
        #[cfg(target_arch = "riscv64")]
        {
            lazy_flush |= self.is_lower_activated();
        }

        Arch::activate(self.root(), lazy_flush);
    }
}

#[cfg(target_os = "none")]
impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> Drop for PageTable64<Arch, PTE> {
    fn drop(&mut self) {
        if self.is_owned() {
            for frame in self.frames.iter() {
                Self::deallocate_frame(*frame);
            }
        }
    }
}

#[cfg(target_os = "none")]
impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTable64<Arch, PTE> {
    const NUM_ENTRIES: usize = 512;

    #[allow(unused)]
    #[inline(always)]
    const fn p4_index(vaddr: usize) -> usize {
        (vaddr >> (12 + 27)) & (Self::NUM_ENTRIES - 1)
    }

    #[inline(always)]
    const fn p3_index(vaddr: usize) -> usize {
        (vaddr >> (12 + 18)) & (Self::NUM_ENTRIES - 1)
    }

    #[inline(always)]
    const fn p2_index(vaddr: usize) -> usize {
        (vaddr >> (12 + 9)) & (Self::NUM_ENTRIES - 1)
    }

    #[inline(always)]
    const fn p1_index(vaddr: usize) -> usize {
        (vaddr >> 12) & (Self::NUM_ENTRIES - 1)
    }
}

#[cfg(target_os = "none")]
impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTable64<Arch, PTE> {
    /// # Safety
    /// This breaks Rust's mutability rule, use it properly
    pub unsafe fn get_entry_internal(
        &self,
        vaddr: VirtualAddress,
    ) -> PagingResult<(&mut PTE, PageSize)> {
        let vaddr = vaddr.as_usize();

        let pt_l3 = if Arch::LEVELS == 3 {
            self.raw_table_of(self.root())
        } else if Arch::LEVELS == 4 {
            let pt_l4 = self.raw_table_of(self.root());
            let pt_l4e = &mut pt_l4[Self::p4_index(vaddr)];
            self.get_next_level(pt_l4e)?
        } else {
            panic!("Unsupported page table");
        };
        let pt_l3e = &mut pt_l3[Self::p3_index(vaddr)];

        if pt_l3e.is_huge() {
            return Ok((pt_l3e, PageSize::_1G));
        }

        let pt_l2 = self.get_next_level(pt_l3e)?;
        let pt_l2e = &mut pt_l2[Self::p2_index(vaddr)];
        if pt_l2e.is_huge() {
            return Ok((pt_l2e, PageSize::_2M));
        }

        let pt_l1 = self.get_next_level(pt_l2e)?;
        let pt_1e = &mut pt_l1[Self::p1_index(vaddr)];
        Ok((pt_1e, PageSize::_4K))
    }

    pub fn get_entry(&self, vaddr: VirtualAddress) -> PagingResult<(&PTE, PageSize)> {
        unsafe {
            self.get_entry_internal(vaddr)
                .map(|(pte, size)| (pte as &_, size))
        }
    }

    pub fn get_entry_mut(&mut self, vaddr: VirtualAddress) -> PagingResult<(&mut PTE, PageSize)> {
        debug_assert!(self.is_owned());

        unsafe { self.get_entry_internal(vaddr) }
    }
}

#[cfg(target_os = "none")]
impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTable64<Arch, PTE> {
    fn allocate_frame() -> PhysicalAddress {
        use address::IPageNum;

        let frame = allocation::alloc_frame().expect("Failed to allocate frame for page table");
        let pa = frame.ppn().start_addr();
        core::mem::forget(frame);

        pa
    }

    fn deallocate_frame(frame: PhysicalAddress) {
        if !frame.is_null() {
            unsafe {
                debug_assert!(frame.is_page_aligned());

                allocation::dealloc_frame_unchecked(frame.to_floor_page_num())
            };
        }
    }

    pub fn allocate() -> Self {
        Self::new(Self::allocate_frame(), true)
    }
}

#[cfg(target_os = "none")]
impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTable64<Arch, PTE> {
    fn get_create_next_level<'a>(&mut self, entry: &mut PTE) -> PagingResult<&'a mut [PTE]> {
        debug_assert!(self.is_owned());

        if entry.is_empty() {
            let frame = Self::allocate_frame();
            self.frames.push(frame);
            *entry = PTE::new_table(frame);

            Ok(self.raw_table_of(frame))
        } else {
            self.get_next_level(entry)
        }
    }

    pub fn get_create_entry(
        &mut self,
        vaddr: VirtualAddress,
        size: PageSize,
    ) -> PagingResult<&mut PTE> {
        debug_assert!(self.is_owned());
        debug_assert!(vaddr.is_page_aligned());

        let vaddr = vaddr.as_usize();

        let pt_l3 = if Arch::LEVELS == 3 {
            self.raw_table_of(self.root())
        } else if Arch::LEVELS == 4 {
            let pt_l4 = self.raw_table_of(self.root());
            let pt_l4e = &mut pt_l4[Self::p4_index(vaddr)];
            self.get_next_level(pt_l4e)?
        } else {
            panic!("Unsupported page table");
        };

        let pt_l3e = &mut pt_l3[Self::p3_index(vaddr)];

        if size == PageSize::_1G {
            return Ok(pt_l3e);
        }

        let pt_l2 = self.get_create_next_level(pt_l3e)?;
        let pt_l2e = &mut pt_l2[Self::p2_index(vaddr)];
        if size == PageSize::_2M {
            return Ok(pt_l2e);
        }

        let p1 = self.get_create_next_level(pt_l2e)?;
        let p1e = &mut p1[Self::p1_index(vaddr)];
        Ok(p1e)
    }
}

#[cfg(target_os = "none")]
impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTable64<Arch, PTE> {
    fn raw_table_of<'a>(&self, paddr: PhysicalAddress) -> &'a mut [PTE] {
        debug_assert!(paddr.is_page_aligned());
        debug_assert!(!paddr.is_null());

        let ptr = unsafe { paddr.to_high_virtual().as_mut_ptr() };
        unsafe { core::slice::from_raw_parts_mut(ptr, Self::NUM_ENTRIES) }
    }

    fn get_next_level<'a>(&self, entry: &PTE) -> PagingResult<&'a mut [PTE]> {
        #[cfg(not(target_arch = "loongarch64"))]
        {
            if !entry.is_present() {
                Err(PagingError::NotMapped)
            } else if entry.is_huge() {
                Err(PagingError::MappedToHugePage)
            } else {
                Ok(self.raw_table_of(entry.paddr()))
            }
        }
        #[cfg(target_arch = "loongarch64")]
        {
            if entry.paddr().is_null() {
                Err(PagingError::NotMapped)
            } else if entry.is_huge() {
                Err(PagingError::MappedToHugePage)
            } else {
                Ok(self.raw_table_of(entry.paddr()))
            }
        }
    }
}

#[cfg(target_os = "none")]
impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTable64<Arch, PTE> {
    pub fn map_single(
        &mut self,
        vaddr: VirtualAddress,
        target: PhysicalAddress,
        size: PageSize,
        flags: GenericMappingFlags,
    ) -> PagingResult<FlushHandle<Arch>> {
        debug_assert!(self.is_owned());
        debug_assert!(vaddr.is_page_aligned());
        debug_assert!(target.is_page_aligned());

        let entry = self.get_create_entry(vaddr, size)?;
        if !entry.is_empty() {
            return Err(PagingError::AlreadyMapped);
        }

        *entry = PTE::new_page(target.page_down(), flags, size != PageSize::_4K);
        Ok(FlushHandle::new(vaddr))
    }

    pub fn remap_single(
        &mut self,
        vaddr: VirtualAddress,
        new_target: PhysicalAddress,
        flags: GenericMappingFlags,
    ) -> PagingResult<(FlushHandle<Arch>, PageSize)> {
        debug_assert!(self.is_owned());
        debug_assert!(vaddr.is_page_aligned());
        debug_assert!(new_target.is_page_aligned());

        let (entry, size) = self.get_entry_mut(vaddr)?;
        entry.set_paddr(new_target);
        entry.set_flags(flags, size != PageSize::_4K);
        Ok((FlushHandle::new(vaddr), size))
    }

    pub fn unmap_single(
        &mut self,
        vaddr: VirtualAddress,
    ) -> PagingResult<(PhysicalAddress, PageSize, FlushHandle<Arch>)> {
        debug_assert!(self.is_owned());
        debug_assert!(vaddr.is_page_aligned());

        let (entry, size) = self.get_entry_mut(vaddr)?;
        if !entry.is_present() {
            entry.clear();
            return Err(PagingError::NotMapped);
        }

        let paddr = entry.paddr();

        entry.clear();

        // Should we deallocate here? The code below causes kernel panic
        // So remove it for now
        // if self.is_owned() {
        //     self.frames.retain(|f| *f != paddr);
        //     Self::deallocate_frame(paddr);
        // }

        Ok((paddr, size, FlushHandle::new(vaddr)))
    }

    pub fn query_virtual(
        &self,
        vaddr: VirtualAddress,
    ) -> PagingResult<(PhysicalAddress, GenericMappingFlags, PageSize)> {
        let (entry, size) = self.get_entry(vaddr.page_down())?;

        if entry.is_empty() {
            return Err(PagingError::NotMapped);
        }

        let offset = vaddr.as_usize() & (size.alignment() - 1);
        Ok((entry.paddr() | offset, entry.flags(), size))
    }
}
