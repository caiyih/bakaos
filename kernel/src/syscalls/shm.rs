use crate::shared_memory;
use abstractions::IUsizeAlias;
use address::{IAlignableAddress, IPageNum, VirtualAddress};
use bitflags::bitflags;
use constants::{SyscallError, PAGE_SIZE};
use page_table::{GenericMappingFlags, IArchPageTableEntry, IArchPageTableEntryBase};
use platform_specific::ISyscallContext;
use tasks::SyscallContext;

use super::{ISyncSyscallHandler, SyscallResult};

pub struct SharedMemoryGetSyscall;

impl ISyncSyscallHandler for SharedMemoryGetSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let key = ctx.arg0::<usize>();

        let shmid = match key {
            0 => shared_memory::get_last_created(),
            _ if shared_memory::is_shm_existing(key) => Ok(key),
            _ => Err(key),
        };

        if let Ok(shmid) = shmid {
            return Ok(shmid as isize);
        }

        let shmflg = ctx.arg2::<usize>();

        if shmflg & 0o1000 > 0 {
            let shmid = shmid.unwrap_err();
            let size = ctx.arg1::<usize>();

            assert!(shared_memory::allocate_at(shmid, size));

            return Ok(shmid as isize);
        }

        SyscallError::NoSuchFileOrDirectory
    }

    fn name(&self) -> &str {
        "sys_shmget"
    }
}

pub struct SharedMemoryAttachSyscall;

impl ISyncSyscallHandler for SharedMemoryAttachSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let shmid = ctx.arg0::<usize>();
        let _shmaddr = ctx.arg1::<VirtualAddress>();
        let _shmflg = ctx.arg2::<usize>();

        match shared_memory::apply_mapping_for(ctx, shmid) {
            None => SyscallError::InvalidArgument,
            Some(page) => Ok(page.start_addr().as_usize() as isize),
        }
    }

    fn name(&self) -> &str {
        "sys_shmat"
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct ProtFlags: u32 {
        const None = 0;
        const Read = 1;
        const Write = 2;
        const Exec = 4;
        const ReadWrite = Self::Read.bits() | Self::Write.bits();
        const All = Self::ReadWrite.bits() | Self::Exec.bits();
    }
}

impl ProtFlags {
    pub fn modify_mapping(&self, mut mapping: GenericMappingFlags) -> GenericMappingFlags {
        mapping.set(GenericMappingFlags::Readable, self.contains(Self::Read));
        mapping.set(GenericMappingFlags::Writable, self.contains(Self::Write));
        mapping.set(GenericMappingFlags::Executable, self.contains(Self::Exec));

        mapping
    }
}

pub struct MemProtectSyscall;

impl ISyncSyscallHandler for MemProtectSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let addr = ctx.arg0::<VirtualAddress>();
        let size = ctx.arg1::<usize>();
        let flags = ctx.arg2::<ProtFlags>();

        log::error!("memprotect: {:?} {:x} {:?}", addr, size, flags);

        if !addr.is_page_aligned() || size % PAGE_SIZE != 0 {
            return SyscallError::InvalidArgument;
        }

        let mut pcb = ctx.pcb.lock();
        let pt = pcb.memory_space.page_table_mut();

        let mut offset = 0;
        while offset < size {
            match pt.get_entry_mut(addr + offset) {
                Err(_) => return SyscallError::InvalidArgument,
                Ok((entry, page_size)) => {
                    #[allow(clippy::identity_op)]
                    let page_size = match page_size {
                        page_table::PageSize::_4K => 4 * 1024,
                        page_table::PageSize::_2M => 2 * 1024 * 1024,
                        page_table::PageSize::_1G => 1 * 1024 * 1024 * 1024,
                    };

                    if offset + page_size > size {
                        return SyscallError::InvalidArgument;
                    }

                    let new_flags = flags.modify_mapping(entry.flags());

                    entry.set_flags(new_flags, page_size > 4096);

                    offset += page_size;
                }
            }
        }

        Ok(0)
    }

    fn name(&self) -> &str {
        "sys_mprotect"
    }
}
