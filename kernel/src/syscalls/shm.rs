use crate::shared_memory;
use address::{IPageNum, VirtualAddress};
use constants::SyscallError;

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};

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

        match shared_memory::apply_mapping_for(&ctx, shmid) {
            None => SyscallError::InvalidArgument,
            Some(page) => {
                Ok(unsafe { page.start_addr::<VirtualAddress>().as_ptr::<()>() } as isize)
            }
        }
    }

    fn name(&self) -> &str {
        "sys_shmat"
    }
}
