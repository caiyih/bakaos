use std::sync::Arc;

use abstractions::IUsizeAlias;
use address::{IAddressBase, PhysicalAddress, VirtualAddress, VirtualAddressRange};
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
    test_env_memory: bool,
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
        todo!()
    }

    fn remap_single(
        &mut self,
        vaddr: VirtualAddress,
        new_target: PhysicalAddress,
        flags: GenericMappingFlags,
    ) -> PagingResult<PageSize> {
        todo!()
    }

    fn unmap_single(&mut self, vaddr: VirtualAddress) -> PagingResult<(PhysicalAddress, PageSize)> {
        todo!()
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
        todo!()
    }

    fn translate_continuous(
        &self,
        vaddr: VirtualAddress,
        size: usize,
    ) -> Result<VirtualAddressRange, MMUError> {
        todo!()
    }

    fn translate_page(&self, vaddr: VirtualAddress) -> Result<VirtualAddress, MMUError> {
        todo!()
    }

    fn translate_continuous_paddr(
        &self,
        paddr: PhysicalAddress,
        size: usize,
    ) -> Result<VirtualAddressRange, MMUError> {
        todo!()
    }

    unsafe fn translate_paddr(&self, paddr: PhysicalAddress) -> Result<VirtualAddress, MMUError> {
        todo!()
    }

    fn inspect_bytes(&self, vaddr: VirtualAddress, len: usize) -> Result<&[u8], MMUError> {
        if vaddr.is_null() {
            return Err(MMUError::InvalidAddress);
        }

        let mut current_vaddr = vaddr;
        let end_vaddr = vaddr + len;

        while current_vaddr < end_vaddr {
            let mapping = self
                .query_mapping(current_vaddr)
                .ok_or(MMUError::InvalidAddress)?;

            ensure_permisssion(current_vaddr, mapping.flags, false)?;

            if !mapping.test_env_memory && !self.alloc.lock().check_paddr(mapping.phys, mapping.len) {
                return Err(MMUError::AccessFault);
            }

            let offset_in_mapping = current_vaddr.as_usize() - mapping.virt.as_usize();
            let remaining_in_mapping = mapping.len - offset_in_mapping;

            let remaining_in_request = end_vaddr - current_vaddr;

            let step = remaining_in_mapping.min(remaining_in_request.as_usize());
            current_vaddr += step;
        }

        unsafe { Ok(std::slice::from_raw_parts(vaddr.as_ptr::<u8>(), len)) }
    }

    fn inspect_bytes_mut(&self, vaddr: VirtualAddress, len: usize) -> Result<&mut [u8], MMUError> {
        todo!()
    }

    fn read_bytes(&self, vaddr: VirtualAddress, buf: &mut [u8]) -> Result<(), MMUError> {
        todo!()
    }

    fn write_bytes(&self, vaddr: VirtualAddress, buf: &[u8]) -> Result<(), MMUError> {
        todo!()
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
            test_env_memory: true,
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

fn ensure_permisssion(
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

    if !mutable && flags.contains(GenericMappingFlags::Writable) {
        return Err(MMUError::PageNotWritable { vaddr });
    }

    Ok(())
}
