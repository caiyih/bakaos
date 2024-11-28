use filesystem_abstractions::{FileDescriptorBuilder, PipeBuilder};
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

                match fd_table.allocate(pipe_pair.read_end) {
                    Some(read_end) => guard.read_end = read_end as i32,
                    None => return Err(-1),
                }

                match fd_table.allocate(pipe_pair.write_end) {
                    Some(write_end) => guard.write_end = write_end as i32,
                    None => {
                        fd_table.remove(guard.read_end as usize);
                        return Err(-1);
                    }
                }

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

pub struct DupSyscall;

impl ISyncSyscallHandler for DupSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let fd = ctx.arg0::<usize>();

        let mut fd_table = ctx.tcb.fd_table.lock();
        match fd_table.get(fd) {
            Some(old) => {
                let builder = FileDescriptorBuilder::from_existing(&old);
                match fd_table.allocate(builder) {
                    Some(newfd) => Ok(newfd as isize),
                    None => Err(-1),
                }
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_dup"
    }
}

pub struct Dup3Syscall;

impl ISyncSyscallHandler for Dup3Syscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let oldfd = ctx.arg0::<usize>();
        let newfd = ctx.arg1::<usize>();
        let _flags = ctx.arg2::<usize>();

        if oldfd == newfd {
            return Ok(newfd as isize);
        }

        let mut fd_table = ctx.tcb.fd_table.lock();
        match fd_table.get(oldfd) {
            Some(old) => {
                let builder = FileDescriptorBuilder::from_existing(&old);

                // if newfd is already open, close it
                if fd_table.get(newfd).is_some() {
                    fd_table.remove(newfd);
                }

                match fd_table.allocate_at(builder, newfd) {
                    Some(newfd) => Ok(newfd as isize),
                    None => Err(-1),
                }
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_dup3"
    }
}
