use core::mem::MaybeUninit;

use alloc::sync::Arc;
use constants::{ErrNo, SyscallError};
use filesystem_abstractions::{DirectoryEntryType, FileMetadata};
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use platform_abstractions::ISyscallContext;
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
