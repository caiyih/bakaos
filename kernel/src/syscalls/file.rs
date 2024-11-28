use paging::IWithPageGuardBuilder;

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};

pub struct WriteSyscall;

impl ISyncSyscallHandler for WriteSyscall {
    fn handle(&self, ctx: &mut SyscallContext<'_>) -> SyscallResult {
        let fd = ctx.arg0::<usize>();
        let p_buf = ctx.arg1::<*const u8>();
        let len = ctx.arg2::<usize>();

        let fd = ctx.tcb.fd_table.lock().get(fd);

        if fd.is_none() {
            return Err(-1);
        }

        let fd = fd.unwrap();

        if !fd.can_write() {
            return Err(-1);
        }

        let buf = unsafe { core::slice::from_raw_parts(p_buf, len) };

        match ctx
            .tcb
            .borrow_page_table()
            .guard_slice(buf)
            .mustbe_user()
            .with_read()
        {
            Some(guard) => Ok(fd.file_handle().access().write(&guard) as isize),
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_write"
    }
}
