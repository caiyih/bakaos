use crate::legacy_print;

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};

pub struct WriteSyscall;

impl ISyncSyscallHandler for WriteSyscall {
    // FIXME: should use the file descriptor to write to the correct file
    fn handle(&self, ctx: &mut SyscallContext<'_>) -> SyscallResult {
        let _fd = ctx.arg0::<i32>();
        let buf = ctx.arg1::<*const u8>();
        let len = ctx.arg2::<usize>();

        for i in 0..len {
            let c = unsafe { buf.add(i).read() };
            legacy_print!("{}", c as char);
        }

        Ok(len as isize)
    }

    fn name(&self) -> &str {
        "sys_write"
    }
}
