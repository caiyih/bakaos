use abstractions::IUsizeAlias;
use address::{
    IAddressBase, IPageNum, IToPageNum, VirtualAddress, VirtualAddressRange, VirtualPageNum,
    VirtualPageNumRange,
};
use alloc::{string::String, sync::Arc, vec::Vec};
use allocation_abstractions::IFrameAllocator;
use hermit_sync::SpinMutex;
use log::trace;
use memory_space::{AreaType, MapType, MappingArea, MemorySpace, MemorySpaceAttribute};
use mmu_abstractions::{GenericMappingFlags, IMMU};
use utilities::InvokeOnDrop;
use xmas_elf::{program::ProgramHeader, ElfFile};

use crate::{auxv::AuxVecKey, ILoadExecutable, LinuxLoader, LoadError, ProcessContext};

impl<'a> LinuxLoader<'a> {
    pub fn from_elf(
        elf_data: &impl ILoadExecutable,
        path: &str,
        mut ctx: ProcessContext<'a>,
        mmu: &Arc<SpinMutex<dyn IMMU>>,
        alloc: &Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Result<Self, LoadError> {
        let mut memory_space = MemorySpace::new(mmu.clone(), alloc.clone());
        let mut attr = MemorySpaceAttribute::default();

        // see https://github.com/caiyih/bakaos/issues/26
        let boxed_elf_holding;

        let boxed_elf;

        let elf_info = {
            let required_frames = elf_data.len().div_ceil(constants::PAGE_SIZE);

            let frames = alloc
                .lock()
                .alloc_contiguous(required_frames)
                .ok_or(LoadError::InsufficientMemory)?;

            boxed_elf_holding = InvokeOnDrop::transform(frames, |f| alloc.lock().dealloc_range(f));

            let pt = mmu.lock();

            let slice = pt
                .translate_phys(
                    boxed_elf_holding.start,
                    boxed_elf_holding.end.as_usize() - boxed_elf_holding.start.as_usize(),
                )
                .unwrap();

            let len = elf_data
                .read_at(0, slice)
                .map_err(|_| LoadError::UnableToReadExecutable)?;

            boxed_elf = &mut slice[..len];

            ElfFile::new(boxed_elf).map_err(|_| LoadError::NotElf)?
        };

        // No need to check the ELF magic number because it is already checked in `ElfFile::new`
        // let elf_magic = elf_header.pt1.magic;
        // '\x7fELF' in ASCII
        // const ELF_MAGIC: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];

        let mut min_start_vpn = VirtualPageNum::from_usize(usize::MAX);
        let mut max_end_vpn = VirtualPageNum::from_usize(0);

        let mut implied_ph = VirtualAddress::null();
        let mut phdr = VirtualAddress::null();

        let mut interpreters = Vec::new();

        let mut pie_offset = 0;

        for ph in elf_info.program_iter() {
            trace!("Found program header: {ph:?}");

            match ph.get_type() {
                Ok(xmas_elf::program::Type::Load) => trace!("Loading"),
                Ok(xmas_elf::program::Type::Interp) => {
                    interpreters.push(ph);
                    trace!("Handle later");
                    continue;
                }
                Ok(xmas_elf::program::Type::Phdr) => {
                    phdr = VirtualAddress::from_usize(ph.virtual_addr() as usize);
                    trace!("Handled");
                    continue;
                }
                _ => {
                    trace!("skipping");
                    continue;
                }
            }

            let mut start = VirtualAddress::from_usize(ph.virtual_addr() as usize);
            let mut end = start + ph.mem_size() as usize;

            if start.to_floor_page_num().as_usize() == 0 {
                pie_offset = constants::PAGE_SIZE;
            }

            if pie_offset != 0 {
                start += pie_offset;
                end += pie_offset;
            }

            if implied_ph.is_null() {
                implied_ph = start;
            }

            min_start_vpn = min_start_vpn.min(start.to_floor_page_num());
            max_end_vpn = max_end_vpn.max(end.to_floor_page_num());

            let mut segment_permissions = GenericMappingFlags::User | GenericMappingFlags::Kernel;

            if ph.flags().is_read() {
                segment_permissions |= GenericMappingFlags::Readable;
            }

            if ph.flags().is_write() {
                segment_permissions |= GenericMappingFlags::Writable;
            }

            if ph.flags().is_execute() {
                segment_permissions |= GenericMappingFlags::Executable;
            }

            let page_range = VirtualPageNumRange::from_start_end(
                start.to_floor_page_num(),
                end.to_ceil_page_num(), // end is exclusive
            );

            memory_space.alloc_and_map_area(MappingArea::new(
                page_range,
                AreaType::UserElf,
                MapType::Framed,
                segment_permissions,
                None,
            ));

            fn copy_elf_segment(
                elf: &[u8],
                ph: &ProgramHeader,
                vaddr: VirtualAddress,
                mmu: &Arc<SpinMutex<dyn IMMU>>,
            ) -> Result<(), LoadError> {
                let file_sz = ph.file_size() as usize;

                if file_sz > 0 {
                    let off = ph.offset() as usize;
                    let end = off.checked_add(file_sz).ok_or(LoadError::TooLarge)?;
                    if end > elf.len() {
                        return Err(LoadError::IncompleteExecutable);
                    }
                    let data = &elf[off..end];
                    mmu.lock()
                        .write_bytes(vaddr, data)
                        .map_err(|_| LoadError::FailedToLoad)?;
                }

                Ok(())
            }

            copy_elf_segment(boxed_elf, &ph, start, mmu)?;
        }

        for interp in interpreters {
            log::warn!("interpreter found: {interp:?}")
            // TODO
        }

        // TODO: investigate this, certain section starts with the va of 0
        // e.g. testcase basic brk
        // debug_assert!(min_start_vpn > VirtualPageNum::from_usize(0));
        min_start_vpn = min_start_vpn.max(VirtualPageNum(1));

        attr.elf_area = VirtualAddressRange::from_start_end(
            min_start_vpn.start_addr(),
            max_end_vpn.start_addr(),
        );

        log::debug!("Elf segments loaded, max_end_vpn: {max_end_vpn:?}");

        if phdr.is_null() {
            phdr = implied_ph + elf_info.header.pt2.ph_offset() as usize
        }

        ctx.auxv.insert(AuxVecKey::AT_PHDR, phdr.as_usize());
        ctx.auxv.insert(
            AuxVecKey::AT_PHENT,
            elf_info.header.pt2.ph_entry_size() as usize,
        );
        ctx.auxv
            .insert(AuxVecKey::AT_PHNUM, elf_info.header.pt2.ph_count() as usize);
        ctx.auxv.insert(AuxVecKey::AT_PAGESZ, constants::PAGE_SIZE);
        ctx.auxv.insert(AuxVecKey::AT_BASE, 0); // FIXME: correct value
        ctx.auxv.insert(AuxVecKey::AT_FLAGS, 0);
        ctx.auxv.insert(
            AuxVecKey::AT_ENTRY, // always the main program's entry point
            elf_info.header.pt2.entry_point() as usize,
        );

        // Reserved for signal trampoline
        max_end_vpn += 1;
        attr.signal_trampoline = max_end_vpn;

        max_end_vpn += 1;
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtualPageNumRange::from_single(max_end_vpn),
            AreaType::UserStackGuardBase,
            MapType::Framed,
            GenericMappingFlags::empty(),
            None,
        ));
        attr.stack_guard_base =
            VirtualAddressRange::from_start_len(max_end_vpn.start_addr(), constants::PAGE_SIZE);

        let stack_page_count = constants::USER_STACK_SIZE / constants::PAGE_SIZE;
        max_end_vpn += 1;
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtualPageNumRange::from_start_count(max_end_vpn, stack_page_count),
            AreaType::UserStack,
            MapType::Framed,
            GenericMappingFlags::User
                .union(GenericMappingFlags::Readable)
                .union(GenericMappingFlags::Writable),
            None,
        ));
        attr.stack_range = VirtualAddressRange::from_start_len(
            max_end_vpn.start_addr(),
            constants::USER_STACK_SIZE,
        );

        max_end_vpn += stack_page_count;
        let stack_top = max_end_vpn.start_addr();
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtualPageNumRange::from_single(max_end_vpn),
            AreaType::UserStackGuardTop,
            MapType::Framed,
            GenericMappingFlags::empty(),
            None,
        ));
        attr.stack_guard_top =
            VirtualAddressRange::from_start_len(max_end_vpn.start_addr(), constants::PAGE_SIZE);

        max_end_vpn += 1;
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtualPageNumRange::from_start_count(max_end_vpn, 0),
            AreaType::UserBrk,
            MapType::Framed,
            GenericMappingFlags::User
                .union(GenericMappingFlags::Readable)
                .union(GenericMappingFlags::Writable),
            None,
        ));
        attr.brk_area_idx = memory_space
            .mappings()
            .iter()
            .enumerate()
            .find(|(_, area)| area.area_type == AreaType::UserBrk)
            .expect("UserBrk area not found")
            .0;
        attr.brk_start = max_end_vpn.start_addr();

        // FIXME: handle cases where there is a interpreter
        let entry_pc =
            VirtualAddress::from_usize(elf_info.header.pt2.entry_point() as usize) + pie_offset;

        #[cfg(debug_assertions)]
        {
            for area in memory_space.mappings() {
                let start = area.range.start().start_addr();
                let end = area.range.end().start_addr();

                let area_type = area.area_type;

                log::trace!("{area_type:?}: {start}..{end}");
            }

            let trampoline_page = attr.signal_trampoline;
            log::trace!(
                "SignalTrampoline: {}..{}",
                trampoline_page.start_addr(),
                trampoline_page.end_addr()
            );
        }

        unsafe {
            memory_space.init(attr);
        }

        Ok(LinuxLoader {
            memory_space,
            entry_pc,
            stack_top,
            argv_base: stack_top,
            envp_base: stack_top,
            ctx,
            executable: String::from(path),
        })
    }
}
