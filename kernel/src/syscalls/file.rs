use filesystem_abstractions::PipeBuilder;
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

pub struct Pipe2Syscall;

impl ISyncSyscallHandler for Pipe2Syscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        struct FdPair {
            read_end: i32,
            write_end: i32,
        }

        let p_fd = ctx.arg0::<*mut FdPair>();

        match ctx
            .tcb
            .borrow_page_table()
            .guard_ptr(p_fd)
            .mustbe_user()
            .with_write()
        {
            Some(mut guard) => {
                let pipe_pair = PipeBuilder::open();

                let mut fd_table = ctx.tcb.fd_table.lock();

                guard.read_end = fd_table.allocate(pipe_pair.read_end) as i32;
                guard.write_end = fd_table.allocate(pipe_pair.write_end) as i32;

                Ok(0)
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_pipe2"
    }
}

pub struct CloseSyscall;

impl ISyncSyscallHandler for CloseSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let fd = ctx.arg0::<usize>();

        ctx.tcb.fd_table.lock().remove(fd); // rc to file will be dropped as the fd is removed
                                            // and when rc is 0, the opened file will be dropped

        Ok(0)
    }

    fn name(&self) -> &str {
        "sys_close"
    }
}
