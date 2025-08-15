use abstractions::IUsizeAlias;
use address::{
    IAddressBase, IAlignableAddress, IPageNum, IToPageNum, VirtualAddress, VirtualPageNumRange,
};
use alloc::vec::Vec;
use constants::SyscallError;
use memory_space_abstractions::{AreaType, MapType, MappingArea, MemorySpace};
use mmap_abstractions::{MemoryMapFlags, MemoryMapProt};
use mmu_abstractions::GenericMappingFlags;

use crate::{SyscallContext, SyscallResult};

impl SyscallContext {
    const VMA_MAX_LEN: usize = 1 << 36; // 64 GB
    const VMA_MIN_ADDR: VirtualAddress = VirtualAddress::from_usize(0x1000);
    const VMA_BASE: VirtualAddress = VirtualAddress::from_usize(0x10000000);
    const VMA_GAP: usize = constants::PAGE_SIZE;

    pub fn sys_mmap(
        &self,
        addr: VirtualAddress,
        len: usize,
        prot: MemoryMapProt,
        flags: MemoryMapFlags,
        #[expect(unused)] // we don't use fd for anonymous mapping
        fd: usize,
        offset: usize,
    ) -> SyscallResult {
        if !addr.is_page_aligned() || (!addr.is_null() && addr < Self::VMA_MIN_ADDR) {
            return SyscallError::BadAddress;
        }

        if len > Self::VMA_MAX_LEN {
            return SyscallError::CannotAllocateMemory;
        }

        if offset % constants::PAGE_SIZE != 0 {
            return SyscallError::InvalidArgument;
        }

        if len == 0 || len % constants::PAGE_SIZE != 0 {
            return SyscallError::InvalidArgument;
        }

        let permissions = Self::prot_to_permissions(prot);

        match flags {
            MemoryMapFlags::ANONYMOUS => self.sys_mmap_anonymous(addr, len, permissions, offset),
            _ => SyscallError::InvalidArgument, // not implemented
        }
    }

    fn sys_mmap_anonymous(
        &self,
        mut addr: VirtualAddress,
        len: usize,
        permissions: GenericMappingFlags,
        offset: usize,
    ) -> SyscallResult {
        // ensure offset is valid
        // some implementations require fd to be -1 for anonymous mapping, but we don't
        if offset != 0 {
            return SyscallError::InvalidArgument;
        }

        let mut mem = self.task.process().memory_space().lock();

        addr = Self::sys_mmap_select_addr(&mut mem, addr, len);

        // No avaliable address
        if addr.is_null() {
            return SyscallError::CannotAllocateMemory;
        }

        let start = addr.to_floor_page_num();
        let end = (addr + len).to_ceil_page_num();

        mem.alloc_and_map_area(MappingArea {
            range: VirtualPageNumRange::from_start_end(start, end),
            area_type: AreaType::VMA,
            map_type: MapType::Framed,
            permissions,
            allocation: None,
        });

        Ok(addr.as_usize() as isize)
    }

    fn sys_mmap_select_addr(
        mem: &mut MemorySpace,
        addr: VirtualAddress,
        len: usize,
    ) -> VirtualAddress {
        debug_assert!(len % constants::PAGE_SIZE == 0);

        let mut mappings = mem.mappings().iter().collect::<Vec<_>>();
        mappings.sort_by(|lhs, rhs| lhs.range().end().cmp(&rhs.range().end()));

        // Try find the first avaliable hole
        let mut last_hole_start = match (addr.is_null(), mappings.len()) {
            (false, 0) => return addr,
            (true, 0) => return Self::VMA_BASE,
            // We start from a mapping's end to avoid overlap with it
            (true, _) => mappings[0].range().end().end_addr() + Self::VMA_GAP,
            _ => addr, // search from the given address
        };

        for mapping in mappings.iter() {
            let mapping_range = mapping.range();
            let possible_hole = VirtualPageNumRange::from_start_count(
                last_hole_start.to_ceil_page_num(),
                len / constants::PAGE_SIZE,
            );

            if mapping_range.contains(possible_hole.start())
                || mapping_range.contains(possible_hole.end())
            {
                last_hole_start = mapping_range.end().end_addr() + Self::VMA_GAP;
                continue;
            }

            if possible_hole.end().end_addr() + Self::VMA_GAP <= mapping_range.start().start_addr()
            {
                return last_hole_start;
            }
        }

        mappings.last().unwrap().range().end().end_addr() + Self::VMA_GAP
    }

    fn prot_to_permissions(prot: MemoryMapProt) -> GenericMappingFlags {
        let mut flags = GenericMappingFlags::User;

        if prot.contains(MemoryMapProt::READ) {
            flags |= GenericMappingFlags::Readable;
        }

        if prot.contains(MemoryMapProt::WRITE) {
            flags |= GenericMappingFlags::Writable;
        }

        if prot.contains(MemoryMapProt::EXECUTE) {
            flags |= GenericMappingFlags::Executable;
        }

        flags
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use address::{VirtualAddress, VirtualPageNum};
    use allocation_abstractions::IFrameAllocator;
    use hermit_sync::SpinMutex;
    use kernel_abstractions::IKernel;
    use memory_space_abstractions::{MappingAreaAllocation, MemorySpace};
    use mmap_abstractions::MemoryMapProt;
    use mmu_abstractions::IMMU;
    use test_utilities::{
        allocation::contiguous::TestFrameAllocator, kernel::TestKernel, task::TestProcess,
    };

    use super::*;
    use test_log::test;

    fn setup_kernel_with_memory() -> (
        Arc<dyn IKernel>,
        Arc<SpinMutex<dyn IFrameAllocator>>,
        Arc<SpinMutex<dyn IMMU>>,
    ) {
        const MEMORY_RANGE: usize = 1 * 1024 * 1024 * 1024; // 1 GB

        let (alloc, mmu) = TestFrameAllocator::new_with_mmu(MEMORY_RANGE);

        let kernel = TestKernel::new()
            .with_allocator(Some(alloc.clone()))
            .build();

        (kernel, alloc, mmu)
    }

    fn setup_memory_space() -> MemorySpace {
        let (_, alloc, mmu) = setup_kernel_with_memory();

        MemorySpace::new(mmu, alloc)
    }

    fn setup_syscall_context() -> SyscallContext {
        let (kernel, alloc, mmu) = setup_kernel_with_memory();

        let (_, task) = TestProcess::new()
            .with_memory_space(Some(MemorySpace::new(mmu, alloc)))
            .build();

        SyscallContext::new(task, kernel)
    }

    #[test]
    fn test_prot_to_permissions() {
        let prot = MemoryMapProt::READ | MemoryMapProt::WRITE | MemoryMapProt::EXECUTE;
        let permissions = SyscallContext::prot_to_permissions(prot);

        assert!(permissions.contains(GenericMappingFlags::Readable));
        assert!(permissions.contains(GenericMappingFlags::Writable));
        assert!(permissions.contains(GenericMappingFlags::Executable));
        assert!(permissions.contains(GenericMappingFlags::User));
    }

    #[test]
    fn test_addr_specified() {
        let mut mem = setup_memory_space();

        let specified_addr = VirtualAddress::from_usize(0x10000000);

        let addr = SyscallContext::sys_mmap_select_addr(&mut mem, specified_addr, 0x1000);

        assert_eq!(addr, specified_addr);
    }

    #[test]
    fn test_addr_not_specified_empty_mappings() {
        let mut mem = setup_memory_space();

        let addr = SyscallContext::sys_mmap_select_addr(&mut mem, VirtualAddress::null(), 0x1000);

        assert_eq!(addr, SyscallContext::VMA_BASE);
    }

    #[test]
    fn test_addr_not_specified_start_with_gap() {
        let mut mem = setup_memory_space();

        let end = VirtualPageNum::from_usize(0x1000);

        mem.map_area(MappingArea {
            range: VirtualPageNumRange::from_start_end(
                VirtualPageNum::from_usize(0x1),
                VirtualPageNum::from_usize(0x1000),
            ),
            area_type: AreaType::VMA,
            map_type: MapType::Framed,
            permissions: GenericMappingFlags::User,
            allocation: Some(MappingAreaAllocation::empty(mem.allocator().clone())),
        });

        let addr = SyscallContext::sys_mmap_select_addr(&mut mem, VirtualAddress::null(), 0x1000);

        assert!(addr > end.end_addr());
    }

    #[test]
    fn test_addr_hole_used() {
        let mut mem = setup_memory_space();

        // Since the 'end' is exclusive, we actually need to add one to the end address.
        // | 10: first area start | 11: first area end | 12: gap | 13: hole start | 14: hole end | 15: gap | 16: second area start|
        let first = VirtualPageNumRange::from_start_count(VirtualPageNum::from_usize(0x10), 1);
        let second = VirtualPageNumRange::from_start_count(VirtualPageNum::from_usize(0x16), 1);

        mem.alloc_and_map_area(MappingArea {
            range: first,
            area_type: AreaType::VMA,
            map_type: MapType::Framed,
            permissions: GenericMappingFlags::User,
            allocation: None,
        });

        mem.alloc_and_map_area(MappingArea {
            range: second,
            area_type: AreaType::VMA,
            map_type: MapType::Framed,
            permissions: GenericMappingFlags::User,
            allocation: None,
        });

        let addr = SyscallContext::sys_mmap_select_addr(&mut mem, VirtualAddress::null(), 0x1000);

        // We want the addr to be between the two ranges
        assert!(addr > first.end().end_addr());
        assert!(addr < second.start().start_addr(), "addr: {:?}", addr);

        assert!(
            addr.is_page_aligned(),
            "selected address must be page-aligned"
        );
        // Ensure we honor the configured VMA_GAP from the previous mapping
        assert!(
            addr >= first.end().end_addr() + SyscallContext::VMA_GAP,
            "address should be at least VMA_GAP past previous mapping end"
        );
    }

    #[test]
    fn test_addr_specified_collision() {
        let mut mem = setup_memory_space();

        let start_addr = VirtualAddress::from_usize(0x2000);
        let end_page = VirtualPageNum::from_usize(0x1000);

        mem.map_area(MappingArea {
            range: VirtualPageNumRange::from_start_end(start_addr.to_floor_page_num(), end_page),
            area_type: AreaType::VMA,
            map_type: MapType::Framed,
            permissions: GenericMappingFlags::User,
            allocation: Some(MappingAreaAllocation::empty(mem.allocator().clone())),
        });

        let addr = SyscallContext::sys_mmap_select_addr(&mut mem, start_addr + 4096, 0x1000);

        assert!(addr > end_page.end_addr());
    }

    #[test]
    fn test_syscall_misaligned_addr() {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            VirtualAddress::from_usize(0x10001),
            4096,
            MemoryMapProt::READ,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        assert_eq!(ret, SyscallError::BadAddress);
    }

    #[test]
    fn test_syscall_invalid_small_addr() {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            VirtualAddress::from_usize(0x1),
            4096,
            MemoryMapProt::READ,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        assert_eq!(ret, SyscallError::BadAddress);
    }

    #[test]
    fn test_syscall_vary_big_len() {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            SyscallContext::VMA_BASE,
            1 << 62,
            MemoryMapProt::READ,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        assert_eq!(ret, SyscallError::CannotAllocateMemory);
    }

    #[test]
    fn test_syscall_misaligned_offset() {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            SyscallContext::VMA_BASE,
            4096,
            MemoryMapProt::READ,
            MemoryMapFlags::ANONYMOUS,
            0,
            1,
        );

        assert_eq!(ret, SyscallError::InvalidArgument);
    }

    #[test]
    fn test_syscall_anonymous_with_offset() {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            SyscallContext::VMA_BASE,
            4096,
            MemoryMapProt::READ,
            MemoryMapFlags::ANONYMOUS,
            0,
            4096,
        );

        assert_eq!(ret, SyscallError::InvalidArgument);
    }

    #[test]
    fn test_syscall_anonymous_success_return_value() {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            SyscallContext::VMA_BASE,
            4096,
            MemoryMapProt::READ,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        assert!(ret.is_ok());

        let vaddr = VirtualAddress::from_usize(ret.unwrap() as usize);

        assert!(!vaddr.is_null());
        assert!(vaddr.is_page_aligned());
    }

    #[test]
    fn test_syscall_anonymous_mapping_exists() {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            SyscallContext::VMA_BASE,
            4096,
            MemoryMapProt::READ,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        let vaddr = VirtualAddress::from_usize(ret.unwrap() as usize);

        let mem = ctx.task.process().memory_space().lock();

        let target_mapping = mem
            .mappings()
            .iter()
            .find(|mapping| mapping.range().start().start_addr() == vaddr);

        assert!(target_mapping.is_some());
    }

    fn create_buffer(len: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(len);

        buf.resize(len, 0);

        buf
    }

    #[test]
    fn test_syscall_anonymous_mapping_can_read() {
        let ctx = setup_syscall_context();

        let len = 8192;

        let ret = ctx.sys_mmap(
            SyscallContext::VMA_BASE,
            len,
            MemoryMapProt::READ,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        let vaddr = VirtualAddress::from_usize(ret.unwrap() as usize);

        let mut buf = create_buffer(len);

        let mmu = ctx.task.process().mmu().lock();

        let mut inspected_len = 0;
        let inspect_result = mmu.inspect_framed(vaddr, len, |mem, offset| {
            inspected_len += mem.len();
            buf[offset..offset + mem.len()].copy_from_slice(mem); // we can read from the memory space

            true
        });

        assert!(inspect_result.is_ok());
        assert_eq!(inspected_len, len);
    }

    #[test]
    fn test_syscall_anonymous_mapping_can_write() {
        let ctx = setup_syscall_context();

        let len = 8192;

        let ret = ctx.sys_mmap(
            SyscallContext::VMA_BASE,
            len,
            MemoryMapProt::READ | MemoryMapProt::WRITE,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        let vaddr = VirtualAddress::from_usize(ret.unwrap() as usize);

        let buf = create_buffer(len);

        let mmu = ctx.task.process().mmu().lock();

        let mut inspected_len = 0;
        let inspect_result = mmu.inspect_framed_mut(vaddr, len, |mem, offset| {
            inspected_len += mem.len();
            mem.copy_from_slice(&buf[offset..offset + mem.len()]); // we can also write to the memory space

            true
        });

        assert!(inspect_result.is_ok());
        assert_eq!(inspected_len, len);
    }

    #[test]
    fn test_syscall_anonymous_mapping_can_not_read_without_prot_read() {
        let ctx = setup_syscall_context();

        let len = 8192;

        let ret = ctx.sys_mmap(
            SyscallContext::VMA_BASE,
            len,
            MemoryMapProt::NONE,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        let vaddr = VirtualAddress::from_usize(ret.unwrap() as usize);

        let mmu = ctx.task.process().mmu().lock();

        let inspect_result = mmu.inspect_framed(vaddr, len, |_, _| true);

        assert!(inspect_result.is_err());
    }

    #[test]
    fn test_syscall_anonymous_mapping_can_not_write_without_prot_write() {
        let ctx = setup_syscall_context();

        let len = 8192;

        let ret = ctx.sys_mmap(
            SyscallContext::VMA_BASE,
            len,
            MemoryMapProt::NONE,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        let vaddr = VirtualAddress::from_usize(ret.unwrap() as usize);

        let mmu = ctx.task.process().mmu().lock();

        let inspect_result = mmu.inspect_framed_mut(vaddr, len, |_, _| true);

        assert!(inspect_result.is_err());
    }

    #[test]
    fn test_syscall_anonymous_content_persists() {
        let ctx = setup_syscall_context();

        let len = 8192;

        let ret = ctx.sys_mmap(
            VirtualAddress::null(),
            len,
            MemoryMapProt::READ | MemoryMapProt::WRITE,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        let vaddr = VirtualAddress::from_usize(ret.unwrap() as usize);

        let mut random_content = create_buffer(len);

        fill_buffer_with_random_bytes(&mut random_content);

        let mmu = ctx.task.process().mmu().lock();

        mmu.write_bytes(vaddr, &random_content).unwrap();

        let mut read_buffer = create_buffer(len);

        mmu.read_bytes(vaddr, &mut read_buffer).unwrap();

        assert_eq!(random_content, read_buffer);
    }

    fn fill_buffer_with_random_bytes(buf: &mut [u8]) {
        use rand::Rng;

        let mut rng = rand::rng();

        rng.fill(buf);
    }

    fn test_syscall_nonsense_flags_return_invalid_argument(flags: MemoryMapFlags) {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            VirtualAddress::null(),
            0x1000,
            MemoryMapProt::READ,
            flags,
            0,
            0,
        );

        assert_eq!(ret, SyscallError::InvalidArgument);
    }

    #[test]
    fn test_syscall_nonsense_flags() {
        test_syscall_nonsense_flags_return_invalid_argument(MemoryMapFlags::from_bits_retain(
            0xdeadbeef,
        ));
    }

    #[test]
    fn test_syscall_composite_flags() {
        test_syscall_nonsense_flags_return_invalid_argument(
            MemoryMapFlags::SHARED | MemoryMapFlags::PRIVATE,
        );
    }

    fn test_invalid_len(len: usize) {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            VirtualAddress::null(),
            len,
            MemoryMapProt::READ,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        assert_eq!(ret, SyscallError::InvalidArgument);
    }

    #[test]
    fn test_syscall_reject_zero_len() {
        test_invalid_len(0);
    }

    #[test]
    fn test_syscall_reject_misaligned_len() {
        test_invalid_len(1);
        test_invalid_len(4097);
    }

    #[test]
    fn test_syscall_can_not_allocate_too_large_len() {
        let ctx = setup_syscall_context();

        let ret = ctx.sys_mmap(
            VirtualAddress::null(),
            usize::MAX & !0xfff,
            MemoryMapProt::READ | MemoryMapProt::WRITE,
            MemoryMapFlags::ANONYMOUS,
            0,
            0,
        );

        assert_eq!(ret, SyscallError::CannotAllocateMemory);
    }
}
