use paging::{IWithPageGuardBuilder, PageTableEntryFlags};

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};
use crate::legacy_print;

pub struct WriteSyscall;

impl ISyncSyscallHandler for WriteSyscall {
    // FIXME: should use the file descriptor to write to the correct file
    fn handle(&self, ctx: &mut SyscallContext<'_>) -> SyscallResult {
        let _fd = ctx.arg0::<i32>();
        let p_buf = ctx.arg1::<*const u8>();
        let len = ctx.arg2::<usize>();

        let buf = unsafe { core::slice::from_raw_parts(p_buf, len) };

        let memory_space = ctx.tcb.memory_space.lock();
        match memory_space
            .page_table()
            .guard_slice(buf)
            .must_have(PageTableEntryFlags::User)
            .with(PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                for c in guard.iter() {
                    legacy_print!("{}", *c as char);
                }

                Ok(guard.len() as isize)
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_write"
    }
}
