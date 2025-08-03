use std::sync::Arc;

use abstractions::IUsizeAlias;
use address::{IAlignableAddress, PhysicalAddress, VirtualAddress};
use hermit_sync::SpinMutex;
use mmu_abstractions::{
    GenericMappingFlags, IPageTable, MMUError, PageSize, PagingError, PagingResult,
};

use crate::allocation::ITestFrameAllocator;

pub struct TestMMU {
    alloc: Arc<SpinMutex<dyn ITestFrameAllocator>>,
    mappings: Vec<MappingRecord>,
}

struct MappingRecord {
    phys: PhysicalAddress,
    virt: VirtualAddress,
    flags: GenericMappingFlags,
    len: usize,
    from_test_env: bool,
}

impl TestMMU {
    pub fn new(alloc: Arc<SpinMutex<dyn ITestFrameAllocator>>) -> Arc<SpinMutex<dyn IPageTable>> {
        Arc::new(SpinMutex::new(Self {
            alloc,
            mappings: Vec::new(),
        }))
    }
}

impl IPageTable for TestMMU {
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

        return Ok((
            mapping.phys + offset,
            mapping.flags,
            PageSize::from(mapping.len),
        ));
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
            let len = mapping_len.min(len - offset);

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

            mmu_ensure_permisssion(checking_vaddr, mapping.flags, false)?;

            let offset = (checking_vaddr - mapping.virt).as_usize();
            let mapping_len = mapping.len - offset;
            let len = mapping_len.min(len - offset);

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

        Ok(unsafe { std::slice::from_raw_parts_mut(ptr as *mut u8, len) })
    }

    fn platform_payload(&self) -> usize {
        panic!("There's no platform payload for test environment")
    }

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
}

impl TestMMU {
    fn query_mapping(&self, vaddr: VirtualAddress) -> Option<&MappingRecord> {
        for mapping in self.mappings.iter() {
            if mapping.virt <= vaddr && vaddr < mapping.virt + mapping.len {
                return Some(mapping);
            }
        }
        None
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
