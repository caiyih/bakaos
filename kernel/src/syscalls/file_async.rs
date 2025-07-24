use core::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

use address::{IAddressBase, VirtualAddress};
use alloc::sync::Arc;
use constants::{ErrNo, SyscallError};
use filesystem_abstractions::{DirectoryEntryType, FileMetadata, IFile, Pipe};
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use threading::yield_now;

use crate::async_syscall;

async_syscall!(sys_write_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let p_buf = ctx.arg1::<usize>();
    let len = ctx.arg2::<usize>();

    let fd = ctx
        .pcb
        .lock()
        .fd_table
        .get(fd)
        .ok_or(ErrNo::BadFileDescriptor)?
        .clone();

    if !fd.can_write() {
        return Err(ErrNo::BadFileDescriptor);
    }

    let file = fd.access();

    while !file.write_avaliable() {
        yield_now().await;
    }

    match ctx
        .borrow_page_table()
        .guard_slice(p_buf as *mut u8, len)
        .mustbe_user()
        .with_read()
    {
        Some(guard) => Ok(file.write(&guard) as isize),
        None => SyscallError::BadAddress,
    }
});

async_syscall!(sys_read_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let fd = ctx
        .pcb
        .lock()
        .fd_table
        .get(fd)
        .ok_or(ErrNo::BadFileDescriptor)?
        .clone();

    if !fd.can_read() {
        return Err(ErrNo::BadFileDescriptor);
    }

    let file = fd.access();

    while !file.read_avaliable() {
        yield_now().await;
    }

    let p_buf = ctx.arg1::<*mut u8>();
    let len = ctx.arg2::<usize>();

    match ctx
        .borrow_page_table()
        .guard_slice(p_buf, len)
        .mustbe_user()
        .mustbe_readable()
        .with_write()
    {
        Some(mut guard) => Ok(file.read(&mut guard) as isize),
        None => SyscallError::BadAddress,
    }
});

#[repr(C)]
struct IoItem {
    p_data: *const u8,
    len: usize,
}

// pointer is not send by default, and can not cross await point
unsafe impl Sync for IoItem {}

async_syscall!(sys_readv_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let fd = ctx
        .pcb
        .lock()
        .fd_table
        .get(fd)
        .ok_or(ErrNo::BadFileDescriptor)?
        .clone();

    if !fd.can_read() {
        return SyscallError::BadFileDescriptor;
    }

    let file = fd.access();

    let iovec_base = ctx.arg1::<*const IoItem>();
    let len = ctx.arg2::<usize>();

    match ctx
        .borrow_page_table()
        .guard_slice(iovec_base, len)
        .mustbe_user()
        .with_read()
    {
        Some(vec_guard) => {
            let mut bytes_read = 0;

            for io in vec_guard.iter() {
                if io.p_data.is_null() || io.len == 0 {
                    continue;
                }

                while !file.write_avaliable() {
                    yield_now().await;
                }

                match ctx
                    .borrow_page_table()
                    .guard_slice(io.p_data, io.len)
                    .mustbe_user()
                    .with_read()
                {
                    Some(mut data_guard) => bytes_read += file.read(&mut data_guard),
                    None => continue,
                }
            }

            Ok(bytes_read as isize)
        }
        None => SyscallError::BadAddress,
    }
});

async_syscall!(sys_writev_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let fd = ctx
        .pcb
        .lock()
        .fd_table
        .get(fd)
        .ok_or(ErrNo::BadFileDescriptor)?
        .clone();

    if !fd.can_write() {
        return SyscallError::BadFileDescriptor;
    }

    let file = fd.access();

    let iovec_base = ctx.arg1::<*const IoItem>();
    let len = ctx.arg2::<usize>();

    match ctx
        .borrow_page_table()
        .guard_slice(iovec_base, len)
        .mustbe_user()
        .with_read()
    {
        Some(vec_guard) => {
            let mut bytes_written = 0;

            for io in vec_guard.iter() {
                if io.p_data.is_null() || io.len == 0 {
                    continue;
                }

                while !file.write_avaliable() {
                    yield_now().await;
                }

                match ctx
                    .borrow_page_table()
                    .guard_slice(io.p_data, io.len)
                    .mustbe_user()
                    .with_read()
                {
                    Some(data_guard) => bytes_written += file.write(&data_guard),
                    None => continue,
                }
            }

            Ok(bytes_written as isize)
        }
        None => SyscallError::BadAddress,
    }
});

async_syscall!(sys_sendfile_async, ctx, {
    // Linux transfers at most 0x7ffff000 bytes
    // see https://www.man7.org/linux/man-pages/man2/sendfile.2.html
    const SENDFILE_MAX_BYTES: usize = 0x7ffff000;
    const BYTES_PER_LOOP: usize = 512;

    let (in_file, out_file) = {
        let pcb = ctx.pcb.lock();
        let (out_fd, in_fd) = (
            pcb.fd_table
                .get(ctx.arg0::<usize>())
                .ok_or(ErrNo::BadFileDescriptor)?, // out_fd
            pcb.fd_table
                .get(ctx.arg1::<usize>())
                .ok_or(ErrNo::BadFileDescriptor)?, // in_fd
        );

        if !out_fd.can_write() || !in_fd.can_read() {
            return SyscallError::BadFileDescriptor;
        }

        (in_fd.access(), out_fd.access())
    };

    fn calculate_size(
        file_meta: &Option<Arc<FileMetadata>>,
        offset: Option<usize>,
        size: usize,
    ) -> usize {
        if let Some(file_meta) = file_meta {
            let offset = offset.unwrap_or_else(|| file_meta.offset());
            file_meta.set_offset(offset);

            let inode = file_meta.inode();
            let inode_meta = inode.metadata();
            if inode_meta.entry_type != DirectoryEntryType::CharDevice {
                return usize::min(inode_meta.size - offset, size);
            }
        }

        SENDFILE_MAX_BYTES
    }

    let poffset = ctx.arg2::<*const usize>();
    let offset = ctx
        .borrow_page_table()
        .guard_ptr(poffset)
        .mustbe_user()
        .with_read()
        .map(|p| *p);

    let in_meta = in_file.metadata();

    // Should use read/write
    if let Some(ref in_meta) = in_meta {
        let in_inode = in_meta.inode();

        if in_inode.metadata().entry_type == DirectoryEntryType::CharDevice {
            return SyscallError::InvalidArgument;
        }
    }

    let size = ctx.arg3::<usize>();
    let mut remaining_bytes = calculate_size(&in_meta, offset, size);

    let mut bytes_written = 0;
    while remaining_bytes != 0 {
        let buf: MaybeUninit<[u8; BYTES_PER_LOOP]> = MaybeUninit::uninit();
        let mut buf: [u8; BYTES_PER_LOOP] = unsafe { core::mem::transmute::<_, _>(buf) };

        while !in_file.read_avaliable() {
            yield_now().await;
        }

        let to_read = usize::min(BYTES_PER_LOOP, remaining_bytes);

        let bytes_read = in_file.read(&mut buf[..to_read]);

        debug_assert!(bytes_read <= to_read); // at least it must be less than SENDBYTES_PER_LOOP

        while !out_file.write_avaliable() {
            yield_now().await;
        }

        bytes_written += out_file.write(&buf[..bytes_read]);

        remaining_bytes -= bytes_read;
    }

    Ok(bytes_written as isize)
});

async_syscall!(sys_pread_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let fd = ctx
        .pcb
        .lock()
        .fd_table
        .get(fd)
        .ok_or(ErrNo::BadFileDescriptor)?
        .clone();

    if !fd.can_read() {
        return Err(ErrNo::BadFileDescriptor);
    }

    let file = fd.access();

    while !file.read_avaliable() {
        yield_now().await;
    }

    let p_buf = ctx.arg1::<*mut u8>();
    let len = ctx.arg2::<usize>();

    let offset = ctx.arg3::<u64>();

    match ctx
        .borrow_page_table()
        .guard_slice(p_buf, len)
        .mustbe_user()
        .mustbe_readable()
        .with_write()
    {
        Some(mut guard) => Ok(file.pread(&mut guard, offset) as isize),
        None => SyscallError::BadAddress,
    }
});

async_syscall!(sys_pwrite_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let p_buf = ctx.arg1::<usize>();
    let len = ctx.arg2::<usize>();

    let fd = ctx
        .pcb
        .lock()
        .fd_table
        .get(fd)
        .ok_or(ErrNo::BadFileDescriptor)?
        .clone();

    if !fd.can_write() {
        return Err(ErrNo::BadFileDescriptor);
    }

    let file = fd.access();

    while !file.write_avaliable() {
        yield_now().await;
    }

    let offset = ctx.arg3::<u64>();

    match ctx
        .borrow_page_table()
        .guard_slice(p_buf as *mut u8, len)
        .mustbe_user()
        .with_read()
    {
        Some(guard) => Ok(file.pwrite(&guard, offset) as isize),
        None => SyscallError::BadAddress,
    }
});

fn is_pipe(fd: &dyn IFile) -> bool {
    fd.is::<Pipe>()
}

async_syscall!(sys_splice, ctx, {
    const BUF_SIZE: usize = 512;

    let fd_in = ctx.arg0::<usize>();
    let off_in = ctx.arg1::<VirtualAddress>();
    let fd_out = ctx.arg2::<usize>();
    let off_out = ctx.arg3::<VirtualAddress>();
    let len = ctx.arg4::<usize>();
    let _flags = ctx.arg5::<u32>();

    if len == 0 {
        return Ok(0);
    }

    let (fd_in, fd_out) = {
        let pcb = ctx.pcb.lock();

        (
            match pcb.fd_table.get(fd_in) {
                Some(fd) if fd.can_read() => fd.access(),
                _ => return SyscallError::BadFileDescriptor,
            },
            match pcb.fd_table.get(fd_out) {
                Some(fd) if fd.can_write() => fd.access(),
                _ => return SyscallError::BadFileDescriptor,
            },
        )
    };

    if !fd_in.can_read() || !fd_out.can_write() {
        return SyscallError::BadFileDescriptor;
    }

    let fd_in_is_pipe = is_pipe(fd_in.deref());
    let fd_out_is_pipe = is_pipe(fd_out.deref());

    // Either one of them is a pipe, and the other is a regular file
    if fd_in_is_pipe == fd_out_is_pipe {
        return SyscallError::InvalidArgument;
    }

    // if the fd is pipe, the corresponding offset must be null
    // if the fd is file, the corresponding offset must be non-null
    if fd_in_is_pipe != off_in.is_null() {
        return SyscallError::InvalidArgument;
    }

    if fd_out_is_pipe != off_out.is_null() {
        return SyscallError::InvalidArgument;
    }

    macro_rules! parse_offset {
        ($offset:ident) => {
            match $offset.is_null() {
                true => None,
                false => match ctx
                    .borrow_page_table()
                    .guard_ptr($offset.as_ptr::<isize>())
                    .mustbe_user()
                    .mustbe_readable()
                    .with_write()
                {
                    Some(offset) => Some(offset),
                    None => return SyscallError::BadAddress,
                },
            }
        };
    }

    let (mut off_in, mut off_out) = (parse_offset!(off_in), parse_offset!(off_out));

    if let Some(off_in) = &off_in {
        let off_in = *off_in.deref();

        if off_in < 0 {
            return Err(-1);
        }

        if let Some(in_inode) = fd_in.inode() {
            if off_in as usize >= in_inode.metadata().size {
                return Ok(0);
            }
        }
    }

    if let Some(off_out) = &off_out {
        let off_out = *off_out.deref();

        if off_out < 0 {
            return Err(-1);
        }
    }

    // Consider allocating buffer on the heap if the buffer is large.
    // Frequent yields can cause massive boxing/unboxing overhead.
    let mut buf = [0u8; BUF_SIZE];

    let mut bytes_transferred = 0;

    while bytes_transferred < len {
        while !fd_in.read_avaliable() {
            if fd_in_is_pipe {
                break;
            }

            yield_now().await;
        }

        let iter_bytes = buf.len().min(len - bytes_transferred);

        let buf = &mut buf[..iter_bytes];

        let bytes_read = match &off_in {
            None => fd_in.read(buf),
            Some(offset) => fd_in.pread(buf, *offset.deref() as u64),
        };

        debug_assert!(bytes_read <= iter_bytes);

        if bytes_read == 0 {
            break;
        }

        if let Some(off_in) = &mut off_in {
            *off_in.deref_mut() += bytes_read as isize;
        }

        while !fd_out.write_avaliable() {
            yield_now().await;
        }

        let buf = &buf[..bytes_read];

        let bytes_written = match &off_out {
            None => fd_out.write(buf),
            Some(offset) => fd_out.pwrite(buf, *offset.deref() as u64),
        };

        debug_assert!(bytes_written <= bytes_read);

        bytes_transferred += bytes_written;

        if let Some(off_out) = &mut off_out {
            *off_out.deref_mut() += bytes_written as isize;
        }

        if bytes_written < bytes_read {
            break;
        }
    }

    Ok(bytes_transferred as isize)
});
