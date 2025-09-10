use std::{
    alloc::Layout,
    collections::BTreeMap,
    sync::{atomic::AtomicUsize, Arc},
};

use abstractions::IUsizeAlias;
use address::{IAddress, IAlignableAddress, PhysicalAddress, VirtualAddress, VirtualAddressRange};
use hermit_sync::SpinMutex;
use mmu_abstractions::{GenericMappingFlags, MMUError, PageSize, PagingError, PagingResult, IMMU};

use crate::allocation::ITestFrameAllocator;

pub struct TestMMU {
    alloc: Arc<SpinMutex<dyn ITestFrameAllocator>>,
    mappings: Vec<MappingRecord>,
    mapped: SpinMutex<BTreeMap<VirtualAddress, MappedMemory>>,
}

unsafe impl Send for TestMMU {}
unsafe impl Sync for TestMMU {}

struct MappingRecord {
    phys: PhysicalAddress,
    virt: VirtualAddress,
    flags: GenericMappingFlags,
    len: usize,
    from_test_env: bool,
}

impl TestMMU {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(alloc: Arc<SpinMutex<dyn ITestFrameAllocator>>) -> Arc<SpinMutex<dyn IMMU>> {
        Arc::new(SpinMutex::new(Self {
            alloc,
            mappings: Vec::new(),
            mapped: SpinMutex::new(BTreeMap::new()),
        }))
    }
}

impl IMMU for TestMMU {
    fn map_single(
        &mut self,
        vaddr: VirtualAddress,
        target: PhysicalAddress,
        size: PageSize,
        flags: GenericMappingFlags,
    ) -> PagingResult<()> {
        paging_ensure_addr_valid(vaddr)?;
        paging_ensure_addr_valid(target)?;

        // Check overlapping
        for mapping in &self.mappings {
            if mapping.virt <= vaddr && vaddr < mapping.virt + mapping.len {
                return Err(PagingError::AlreadyMapped);
            }
        }

        // Add mapping
        self.mappings.push(MappingRecord {
            phys: target,
            virt: vaddr,
            flags,
            len: size.as_usize(),
            from_test_env: false,
        });

        Ok(())
    }

    fn remap_single(
        &mut self,
        vaddr: VirtualAddress,
        new_target: PhysicalAddress,
        flags: GenericMappingFlags,
    ) -> PagingResult<PageSize> {
        paging_ensure_addr_valid(vaddr)?;
        paging_ensure_addr_valid(new_target)?;

        // Find and modify the mapping
        for mapping in self.mappings.iter_mut() {
            if vaddr == mapping.virt {
                mapping.phys = new_target;
                mapping.flags = flags;
                return Ok(PageSize::from(mapping.len));
            }
        }

        Err(PagingError::NotMapped)
    }

    fn unmap_single(&mut self, vaddr: VirtualAddress) -> PagingResult<(PhysicalAddress, PageSize)> {
        match self
            .mappings
            .iter()
            .enumerate()
            .find(|(_, m)| m.virt == vaddr)
        {
            None => Err(PagingError::NotMapped),
            Some((idx, mapping)) => {
                let ret = (mapping.phys, PageSize::from(mapping.len));

                self.mappings.remove(idx);

                Ok(ret)
            }
        }
    }

    fn query_virtual(
        &self,
        vaddr: VirtualAddress,
    ) -> PagingResult<(PhysicalAddress, GenericMappingFlags, PageSize)> {
        let mapping = self.query_mapping(vaddr).ok_or(PagingError::NotMapped)?;
        let offset = (vaddr - mapping.virt).as_usize();

        Ok((
            mapping.phys + offset,
            mapping.flags,
            PageSize::from(mapping.len),
        ))
    }

    fn create_or_update_single(
        &mut self,
        vaddr: VirtualAddress,
        size: PageSize,
        paddr: Option<PhysicalAddress>,
        flags: Option<GenericMappingFlags>,
    ) -> PagingResult<()> {
        paging_ensure_addr_valid(vaddr)?;
        paging_ensure_valid_size(size)?;

        if let Some(paddr) = paddr {
            paging_ensure_addr_valid(paddr)?;
        }

        // Find and update the mapping
        for mapping in self.mappings.iter_mut() {
            if mapping.virt == vaddr && size == PageSize::from(mapping.len) {
                if let Some(paddr) = paddr {
                    mapping.phys = paddr;
                }

                if let Some(flags) = flags {
                    mapping.flags = flags;
                }

                return Ok(());
            }
        }

        Err(PagingError::NotMapped)
    }

    fn inspect_framed_internal(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        callback: &mut dyn FnMut(&[u8], usize) -> bool,
    ) -> Result<(), MMUError> {
        mmu_ensure_addr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut checking_offset = 0;

        while checking_offset < len {
            let mapping = self
                .query_mapping(checking_vaddr)
                .ok_or(MMUError::InvalidAddress)?;

            mmu_ensure_permisssion(checking_vaddr, mapping.flags, false)?;

            let offset = (checking_vaddr - mapping.virt).as_usize();
            let mapping_len = mapping.len - offset;
            let len = mapping_len.min(len - checking_offset);

            if !mapping.from_test_env && !self.alloc.lock().check_paddr(mapping.phys + offset, len)
            {
                return Err(MMUError::AccessFault);
            }

            let ptr = mapping.phys.as_usize() as *const u8;
            let slice = unsafe { std::slice::from_raw_parts(ptr.add(offset), len) };

            if !callback(slice, checking_offset) {
                break;
            }

            checking_offset += len;
            checking_vaddr += len;
        }

        Ok(())
    }

    fn inspect_framed_mut_internal(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        callback: &mut dyn FnMut(&mut [u8], usize) -> bool,
    ) -> Result<(), MMUError> {
        mmu_ensure_addr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut checking_offset = 0;

        while checking_offset < len {
            let mapping = self
                .query_mapping(checking_vaddr)
                .ok_or(MMUError::InvalidAddress)?;

            mmu_ensure_permisssion(checking_vaddr, mapping.flags, true)?;

            let offset = (checking_vaddr - mapping.virt).as_usize();
            let mapping_len = mapping.len - offset;
            let len = mapping_len.min(len - checking_offset);

            if !mapping.from_test_env && !self.alloc.lock().check_paddr(mapping.phys + offset, len)
            {
                return Err(MMUError::AccessFault);
            }

            let ptr = mapping.phys.as_usize() as *mut u8;
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr.add(offset), len) };

            if !callback(slice, checking_offset) {
                break;
            }

            checking_offset += len;
            checking_vaddr += len;
        }

        Ok(())
    }

    fn read_bytes(&self, vaddr: VirtualAddress, buf: &mut [u8]) -> Result<(), MMUError> {
        self.inspect_framed_internal(vaddr, buf.len(), &mut |src, offset| {
            buf[offset..offset + src.len()].copy_from_slice(src);
            true
        })
    }

    fn write_bytes(&self, vaddr: VirtualAddress, buf: &[u8]) -> Result<(), MMUError> {
        self.inspect_framed_mut_internal(vaddr, buf.len(), &mut |dst, offset| {
            dst.copy_from_slice(&buf[offset..offset + dst.len()]);
            true
        })
    }

    fn translate_phys(
        &self,
        paddr: PhysicalAddress,
        len: usize,
    ) -> Result<&'static mut [u8], MMUError> {
        for mapping in self.mappings.iter().filter(|m| m.from_test_env) {
            if paddr >= mapping.phys && paddr < mapping.phys + mapping.len {
                return Ok(unsafe {
                    std::slice::from_raw_parts_mut(paddr.as_usize() as *mut u8, len)
                });
            }
        }

        let alloc = self.alloc.lock();

        if !alloc.check_paddr(paddr, len) {
            return Err(MMUError::AccessFault);
        }

        let ptr= alloc.linear_map(paddr).expect("The test allocator does not support linear mapping. Use contiguous::TestFrameAllocator");

        Ok(unsafe { std::slice::from_raw_parts_mut(ptr, len) })
    }

    fn platform_payload(&self) -> usize {
        panic!("There's no platform payload for test environment")
    }

    #[cfg(not(target_os = "none"))]
    fn register_internal(&mut self, vaddr: VirtualAddress, len: usize, mutable: bool) {
        let mut flags = GenericMappingFlags::User | GenericMappingFlags::Readable;

        if mutable {
            flags |= GenericMappingFlags::Writable
        }

        self.mappings.push(MappingRecord {
            phys: PhysicalAddress::from_usize(vaddr.as_usize()),
            virt: vaddr,
            flags,
            len,
            from_test_env: true,
        });
    }

    #[cfg(not(target_os = "none"))]
    fn unregister_internal(&mut self, vaddr: VirtualAddress) {
        let mut i = 0;

        while i < self.mappings.len() {
            if self.mappings[i].virt == vaddr {
                self.mappings.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn map_buffer_internal(&self, vaddr: VirtualAddress, len: usize) -> Result<&'_ [u8], MMUError> {
        let mem = MappedMemory::alloc(vaddr, len, false);
        let mut mapped = self.mapped.lock();

        if let Some((_, mapped)) = mapped.iter().find(|m| m.1.range().intersects(mem.range())) {
            let offset = vaddr.diff(mapped.range().start()) as usize;

            mapped.add_ref();
            return Ok(unsafe { core::slice::from_raw_parts(mapped.ptr.add(offset), len) });
        }

        let slice = mem.slice_mut();

        self.read_bytes(vaddr, slice)?;
        mapped.insert(vaddr, mem);

        Ok(slice)
    }

    fn map_buffer_mut_internal(
        &self,
        vaddr: VirtualAddress,
        len: usize,
        _force_mut: bool,
    ) -> Result<&'_ mut [u8], MMUError> {
        let mem = MappedMemory::alloc(vaddr, len, true);
        let mut mapped = self.mapped.lock();

        if let Some((_, mapped)) = mapped.iter().find(|m| m.1.range().intersects(mem.range())) {
            if !mapped.mutable {
                // FIXME: is this correct?
                return Err(MMUError::Borrowed);
            }

            let offset = vaddr.diff(mapped.range().start()) as usize;
            mapped.add_ref();

            return Ok(unsafe { core::slice::from_raw_parts_mut(mapped.ptr.add(offset), len) });
        }

        let slice = mem.slice_mut();

        // TODO: Check if the permission matches force_mut
        self.read_bytes(vaddr, slice)?;

        mapped.insert(vaddr, mem);

        Ok(slice)
    }

    fn unmap_buffer(&self, vaddr: VirtualAddress) {
        let mut locked = self.mapped.lock();

        if let Some((_, mapped)) = locked.iter().find(|(_, m)| m.range().contains(vaddr)) {
            if mapped.release() {
                let key = mapped.vaddr;
                let mapped = locked.remove(&key).unwrap();

                if mapped.mutable {
                    // Sync the mapped memory to the physical memory
                    let slice = mapped.slice_mut();

                    let _ = self.write_bytes(vaddr, slice);
                }
            }
        }
    }

    fn map_cross_internal<'a>(
        &'a mut self,
        source: &'a dyn IMMU,
        vaddr: VirtualAddress,
        len: usize,
    ) -> Result<&'a [u8], MMUError> {
        let source = source.downcast_ref::<TestMMU>().unwrap();

        #[allow(deprecated)]
        source.map_buffer_internal(vaddr, len)
    }

    fn map_cross_mut_internal<'a>(
        &'a mut self,
        source: &'a dyn IMMU,
        vaddr: VirtualAddress,
        len: usize,
    ) -> Result<&'a mut [u8], MMUError> {
        let source = source.downcast_ref::<TestMMU>().unwrap();

        #[allow(deprecated)]
        source.map_buffer_mut_internal(vaddr, len, false)
    }

    fn unmap_cross(&mut self, source: &dyn IMMU, vaddr: VirtualAddress) {
        let source = source.downcast_ref::<TestMMU>().unwrap();

        source.unmap_buffer(vaddr);
    }
}

impl TestMMU {
    fn query_mapping(&self, vaddr: VirtualAddress) -> Option<&MappingRecord> {
        self.mappings
            .iter()
            .find(|&mapping| mapping.virt <= vaddr && vaddr < mapping.virt + mapping.len)
            .map(|v| v as _)
    }
}

fn paging_ensure_valid_size(size: PageSize) -> PagingResult<()> {
    if let PageSize::Custom(size) = size {
        if size % constants::PAGE_SIZE != 0 {
            return Err(PagingError::NotAligned);
        }
    }

    Ok(())
}

fn paging_ensure_addr_valid<T: IAlignableAddress>(addr: T) -> PagingResult<()> {
    if !addr.is_page_aligned() {
        return Err(PagingError::NotAligned);
    }

    Ok(())
}

fn mmu_ensure_addr_valid<T: IAlignableAddress>(addr: T) -> Result<(), MMUError> {
    if addr.is_null() {
        return Err(MMUError::InvalidAddress);
    }

    Ok(())
}

fn mmu_ensure_permisssion(
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

struct MappedMemory {
    vaddr: VirtualAddress,
    ptr: *mut u8,
    layout: Layout,
    mutable: bool,
    rc: AtomicUsize,
}

impl MappedMemory {
    fn alloc(vaddr: VirtualAddress, len: usize, mutable: bool) -> Self {
        let layout = Layout::from_size_align(len, constants::PAGE_SIZE).unwrap();

        let ptr = unsafe { std::alloc::alloc(layout) };

        Self {
            vaddr,
            ptr,
            layout,
            mutable,
            rc: AtomicUsize::new(1),
        }
    }

    fn range(&self) -> VirtualAddressRange {
        VirtualAddressRange::from_start_len(self.vaddr, self.layout.size())
    }

    fn slice_mut(&self) -> &'static mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.layout.size()) }
    }

    fn add_ref(&self) {
        self.rc.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn release(&self) -> bool {
        self.rc.fetch_sub(1, std::sync::atomic::Ordering::Relaxed) == 1
    }
}

impl Drop for MappedMemory {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) };
    }
}
