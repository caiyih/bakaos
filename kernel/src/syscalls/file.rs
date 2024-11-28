use paging::IWithPageGuardBuilder;

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};

pub struct WriteSyscall;

impl ISyncSyscallHandler for WriteSyscall {
    fn handle(&self, ctx: &mut SyscallContext<'_>) -> SyscallResult {
        let fd = ctx.arg0::<usize>();
        let p_buf = ctx.arg1::<*const u8>();
        let len = ctx.arg2::<usize>();

        let fd = ctx.tcb.fd_table.lock().get(fd).ok_or(-1isize)?;

        if !fd.can_write() {
            return Err(-1);
        }

        let file = fd.access().ok_or(-1isize)?; // check if file is closed

        let buf = unsafe { core::slice::from_raw_parts(p_buf, len) };

        match ctx
            .tcb
            .borrow_page_table()
            .guard_slice(buf)
            .mustbe_user()
            .with_read()
        {
            Some(guard) => Ok(file.write(&guard) as isize),
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_write"
    }
}
