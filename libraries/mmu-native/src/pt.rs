use core::marker::PhantomData;

use crate::IArchPageTableEntry;
use abstractions::IUsizeAlias;
use address::{
    IAddressBase, IAlignableAddress, IConvertablePhysicalAddress, PhysicalAddress, VirtualAddress,
};
use alloc::{sync::Arc, vec, vec::Vec};
use allocation_abstractions::{FrameDesc, IFrameAllocator};
use hermit_sync::SpinMutex;
use mmu_abstractions::{GenericMappingFlags, MMUError, PageSize, PagingError, PagingResult, IMMU};

pub trait IPageTableArchAttribute {
    const LEVELS: usize;
    const PA_MAX_BITS: usize;
    const VA_MAX_BITS: usize;
    const PA_MAX_ADDR: usize = (1 << Self::PA_MAX_BITS) - 1;
}

pub struct PageTableNative<Arch, PTE>
where
    Arch: IPageTableArchAttribute,
    PTE: IArchPageTableEntry,
{
    root: PhysicalAddress,
    allocation: Option<PageTableAllocation>,
    _marker: PhantomData<(Arch, PTE)>,
}

unsafe impl<A: IPageTableArchAttribute, P: IArchPageTableEntry> Send for PageTableNative<A, P> {}
unsafe impl<A: IPageTableArchAttribute, P: IArchPageTableEntry> Sync for PageTableNative<A, P> {}

struct PageTableAllocation {
    frames: Vec<FrameDesc>,
    allocator: Arc<SpinMutex<dyn IFrameAllocator>>,
}

impl Drop for PageTableAllocation {
    fn drop(&mut self) {
        while let Some(frame) = self.frames.pop() {
            self.allocator.lock().dealloc(frame);
        }
    }
}

impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> IMMU for PageTableNative<Arch, PTE> {
    fn map_single(
        &mut self,
        vaddr: VirtualAddress,
        target: PhysicalAddress,
        size: PageSize,
        flags: GenericMappingFlags,
    ) -> PagingResult<()> {
        if !target.is_page_aligned() {
            return Err(PagingError::NotAligned);
        }

        let entry = self.get_create_entry(vaddr, size)?;
        if !entry.is_empty() {
            return Err(PagingError::AlreadyMapped);
        }

        *entry = PTE::new_page(target.page_down(), flags, size != PageSize::_4K);
        Ok(())
    }

    fn remap_single(
        &mut self,
        vaddr: VirtualAddress,
        new_target: PhysicalAddress,
        flags: GenericMappingFlags,
    ) -> PagingResult<PageSize> {
        if !new_target.is_page_aligned() {
            return Err(PagingError::NotAligned);
        }

        let (entry, size) = self.get_entry_mut(vaddr)?;
        entry.set_paddr(new_target);
        entry.set_flags(flags, size != PageSize::_4K);
        Ok(size)
    }

    fn unmap_single(&mut self, vaddr: VirtualAddress) -> PagingResult<(PhysicalAddress, PageSize)> {
        let (entry, size) = self.get_entry_mut(vaddr)?;
        if !entry.is_present() {
            entry.clear();
            return Err(PagingError::NotMapped);
        }

        let paddr = entry.paddr();

        entry.clear();

        Ok((paddr, size))
    }

    fn query_virtual(
        &self,
        vaddr: VirtualAddress,
    ) -> PagingResult<(PhysicalAddress, GenericMappingFlags, PageSize)> {
        let (entry, size) = self.get_entry(vaddr.page_down())?;

        if entry.is_empty() {
            return Err(PagingError::NotMapped);
        }

        let offset = vaddr.as_usize() & (size.as_usize() - 1);
        Ok((entry.paddr() | offset, entry.flags(), size))
    }

    fn create_or_update_single(
        &mut self,
        vaddr: VirtualAddress,
        size: PageSize,
        paddr: Option<PhysicalAddress>,
        flags: Option<GenericMappingFlags>,
    ) -> PagingResult<()> {
        let entry = self.get_create_entry(vaddr, size)?;

        if let Some(paddr) = paddr {
            entry.set_paddr(paddr);
        }

        if let Some(flags) = flags {
            entry.set_flags(flags, size != PageSize::_4K);
        }

        Ok(())
    }

    fn platform_payload(&self) -> usize {
        self.root.as_usize()
    }

    fn read_bytes(&self, vaddr: VirtualAddress, buf: &mut [u8]) -> Result<(), MMUError> {
        let mut bytes_read = 0;
        self.inspect_bytes_through_linear(vaddr, buf.len(), |src| {
            buf[bytes_read..bytes_read + src.len()].copy_from_slice(src);

            bytes_read += src.len();
            true
        })
    }

    fn write_bytes(&self, vaddr: VirtualAddress, buf: &[u8]) -> Result<(), MMUError> {
        let mut bytes_written = 0;
        self.inspect_bytes_through_linear(vaddr, buf.len(), |dst| {
            dst.copy_from_slice(&buf[bytes_written..bytes_written + dst.len()]);

            bytes_written += dst.len();
            true
        })
    }

    fn inspect_framed_internal(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        callback: &mut dyn FnMut(&[u8], usize) -> bool,
    ) -> Result<(), MMUError> {
        ensure_vaddr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut remaining_len = len;

        loop {
            let (paddr, flags, size) = self.query_virtual(checking_vaddr).map_err(|e| e.into())?;

            ensure_permission(vaddr, flags, false)?;

            let frame_base = paddr.align_down(size.as_usize());

            let frame_remain_len = size.as_usize() - (paddr.as_usize() - frame_base.as_usize());

            let avaliable_len = remaining_len.min(frame_remain_len);

            let slice = unsafe {
                core::slice::from_raw_parts(
                    // query_virtual adds offset internally
                    paddr.to_high_virtual().as_mut::<u8>(),
                    avaliable_len,
                )
            };

            if !callback(slice, avaliable_len) {
                break;
            }

            checking_vaddr += frame_remain_len;
            remaining_len -= avaliable_len;

            if remaining_len == 0 {
                break;
            }
        }

        Ok(())
    }

    fn inspect_framed_mut_internal(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        callback: &mut dyn FnMut(&mut [u8], usize) -> bool,
    ) -> Result<(), MMUError> {
        ensure_vaddr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut remaining_len = len;

        loop {
            let (paddr, flags, size) = self.query_virtual(checking_vaddr).map_err(|e| e.into())?;

            ensure_permission(vaddr, flags, true)?;

            let frame_base = paddr.align_down(size.as_usize());

            let frame_remain_len = size.as_usize() - (paddr.as_usize() - frame_base.as_usize());

            let avaliable_len = remaining_len.min(frame_remain_len);

            let slice = unsafe {
                core::slice::from_raw_parts_mut(
                    // query_virtual adds offset internally
                    paddr.to_high_virtual().as_mut::<u8>(),
                    avaliable_len,
                )
            };

            if !callback(slice, avaliable_len) {
                break;
            }

            checking_vaddr += frame_remain_len;
            remaining_len -= avaliable_len;

            if remaining_len == 0 {
                break;
            }
        }

        Ok(())
    }

    fn translate_phys(
        &self,
        paddr: PhysicalAddress,
        len: usize,
    ) -> Result<&'static mut [u8], MMUError> {
        let virt = paddr.to_high_virtual();

        Ok(unsafe { core::slice::from_raw_parts_mut(virt.as_mut::<u8>(), len) })
    }

    fn map_buffer_internal(&self, vaddr: VirtualAddress, len: usize) -> Result<&'_ [u8], MMUError> {
        self.inspect_permission(vaddr, len, false)?;

        Ok(unsafe { core::slice::from_raw_parts(vaddr.as_ptr(), len) })
    }

    fn map_buffer_mut_internal(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        _force_mut: bool,
    ) -> Result<&'_ mut [u8], MMUError> {
        self.inspect_permission(vaddr, len, true)?;

        Ok(unsafe { core::slice::from_raw_parts_mut(vaddr.as_mut_ptr(), len) })
    }

    fn unmap_buffer(&self, _vaddr: VirtualAddress) {}
}

impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTableNative<Arch, PTE> {
    fn inspect_permission(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        mutable: bool,
    ) -> Result<(), MMUError> {
        ensure_vaddr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut remaining_len = len;

        loop {
            let (paddr, flags, size) = self.query_virtual(checking_vaddr).map_err(|e| e.into())?;

            ensure_permission(vaddr, flags, mutable)?;

            let frame_base = paddr.align_down(size.as_usize());

            let frame_remain_len = size.as_usize() - (paddr.as_usize() - frame_base.as_usize());
            let avaliable_len = remaining_len.min(frame_remain_len);

            checking_vaddr += frame_remain_len;
            remaining_len -= avaliable_len;

            if remaining_len == 0 {
                break;
            }
        }

        Ok(())
    }

    fn inspect_bytes_through_linear(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        mut callback: impl FnMut(&mut [u8]) -> bool,
    ) -> Result<(), MMUError> {
        ensure_vaddr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut remaining_len = len;

        loop {
            let (paddr, flags, size) = self.query_virtual(checking_vaddr).map_err(|e| e.into())?;

            ensure_linear_permission(flags)?;

            let frame_base = paddr.align_down(size.as_usize());

            let frame_remain_len = size.as_usize() - (paddr.as_usize() - frame_base.as_usize());
            let avaliable_len = remaining_len.min(frame_remain_len);

            {
                let slice = unsafe {
                    core::slice::from_raw_parts_mut(
                        paddr.to_high_virtual().as_mut::<u8>(),
                        avaliable_len,
                    )
                };

                if !callback(slice) {
                    return Ok(());
                }
            }

            checking_vaddr += frame_remain_len;
            remaining_len -= avaliable_len;

            if remaining_len == 0 {
                break;
            }
        }

        Ok(())
    }
}

const fn ensure_vaddr_valid(vaddr: VirtualAddress) -> Result<(), MMUError> {
    if vaddr.is_null() {
        return Err(MMUError::InvalidAddress);
    }

    Ok(())
}

const fn ensure_linear_permission(flags: GenericMappingFlags) -> Result<(), MMUError> {
    if !flags.contains(GenericMappingFlags::User) {
        return Err(MMUError::PrivilegeError);
    }

    Ok(())
}

const fn ensure_permission(
    vaddr: VirtualAddress,
    flags: GenericMappingFlags,
    mutable: bool,
) -> Result<(), MMUError> {
    if !flags.contains(GenericMappingFlags::User) {
        return Err(MMUError::PrivilegeError);
    }

    if !flags.contains(GenericMappingFlags::Readable) {
        return Err(MMUError::PageNotReadable { vaddr });
    }

    if mutable && !flags.contains(GenericMappingFlags::Writable) {
        return Err(MMUError::PageNotWritable { vaddr });
    }

    Ok(())
}

impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTableNative<Arch, PTE> {
    const fn from_borrowed(root: PhysicalAddress) -> Self {
        Self {
            root,
            allocation: None,
            _marker: PhantomData,
        }
    }

    pub fn new(
        root: PhysicalAddress,
        allocator: Option<Arc<SpinMutex<dyn IFrameAllocator>>>,
    ) -> Self {
        match allocator {
            None => Self::from_borrowed(root),
            Some(allocator) => Self {
                root,
                allocation: Some(PageTableAllocation {
                    frames: Vec::new(),
                    allocator,
                }),
                _marker: PhantomData,
            },
        }
    }

    pub fn alloc(allocator: Arc<SpinMutex<dyn IFrameAllocator>>) -> Self {
        let frame = allocator.lock().alloc_frame().unwrap();

        let mut pt = Self::from_borrowed(frame.0);

        pt.allocation = Some(PageTableAllocation {
            frames: vec![frame],
            allocator,
        });

        pt
    }

    const fn root(&self) -> PhysicalAddress {
        self.root
    }

    fn ensure_can_modify(&self) -> PagingResult<&PageTableAllocation> {
        match self.allocation {
            None => Err(PagingError::CanNotModify),
            Some(ref alloc) => Ok(alloc),
        }
    }

    fn ensure_can_modify_mut(&mut self) -> PagingResult<&mut PageTableAllocation> {
        match self.allocation {
            None => Err(PagingError::CanNotModify),
            Some(ref mut alloc) => Ok(alloc),
        }
    }

    /// # Safety
    /// This breaks Rust's mutability rule, use it properly
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_entry_internal(
        &self,
        vaddr: VirtualAddress,
    ) -> PagingResult<(&mut PTE, PageSize)> {
        let vaddr = vaddr.as_usize();

        let pt_l3 = if Arch::LEVELS == 3 {
            self.raw_table_of(self.root())?
        } else if Arch::LEVELS == 4 {
            let pt_l4 = self.raw_table_of(self.root())?;
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

    fn raw_table_of<'a>(&self, paddr: PhysicalAddress) -> PagingResult<&'a mut [PTE]> {
        if !paddr.is_page_aligned() {
            return Err(PagingError::NotAligned);
        }

        if paddr.is_null() {
            return Err(PagingError::NotMapped);
        }

        let ptr = unsafe { paddr.to_high_virtual().as_mut_ptr() };
        Ok(unsafe { core::slice::from_raw_parts_mut(ptr, Self::NUM_ENTRIES) })
    }

    fn get_next_level<'a>(&self, entry: &PTE) -> PagingResult<&'a mut [PTE]> {
        #[cfg(not(target_arch = "loongarch64"))]
        {
            if !entry.is_present() {
                Err(PagingError::NotMapped)
            } else if entry.is_huge() {
                Err(PagingError::MappedToHugePage)
            } else {
                self.raw_table_of(entry.paddr())
            }
        }
        #[cfg(target_arch = "loongarch64")]
        {
            if entry.paddr().is_null() {
                Err(PagingError::NotMapped)
            } else if entry.is_huge() {
                Err(PagingError::MappedToHugePage)
            } else {
                self.raw_table_of(entry.paddr())
            }
        }
    }

    fn get_entry(&self, vaddr: VirtualAddress) -> PagingResult<(&PTE, PageSize)> {
        unsafe {
            self.get_entry_internal(vaddr)
                .map(|(pte, size)| (pte as &_, size))
        }
    }

    fn get_entry_mut(&mut self, vaddr: VirtualAddress) -> PagingResult<(&mut PTE, PageSize)> {
        let _ = self.ensure_can_modify()?;

        unsafe { self.get_entry_internal(vaddr) }
    }

    fn get_create_entry(
        &mut self,
        vaddr: VirtualAddress,
        size: PageSize,
    ) -> PagingResult<&mut PTE> {
        let _ = self.ensure_can_modify()?;
        if !vaddr.is_page_aligned() {
            return Err(PagingError::NotAligned);
        }

        let vaddr = vaddr.as_usize();

        let pt_l3 = if Arch::LEVELS == 3 {
            self.raw_table_of(self.root())?
        } else if Arch::LEVELS == 4 {
            let pt_l4 = self.raw_table_of(self.root())?;
            let pt_l4e = &mut pt_l4[Self::p4_index(vaddr)];
            self.get_create_next_level(pt_l4e)?
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

    fn get_create_next_level<'a>(&mut self, entry: &mut PTE) -> PagingResult<&'a mut [PTE]> {
        let alloc = self.ensure_can_modify_mut()?;

        if entry.is_empty() {
            let frame = alloc
                .allocator
                .lock()
                .alloc_frame()
                .ok_or(PagingError::OutOfMemory)?;

            let paddr = frame.0;

            alloc.frames.push(frame);
            *entry = PTE::new_table(paddr);

            self.raw_table_of(paddr)
        } else {
            self.get_next_level(entry)
        }
    }
}

impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTableNative<Arch, PTE> {
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
