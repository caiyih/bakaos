use core::mem::MaybeUninit;

use alloc::sync::Arc;
use filesystem_abstractions::FileMetadata;
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use threading::yield_now;

use crate::async_syscall;

async_syscall!(sys_write_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let p_buf = ctx.arg1::<usize>();
    let len = ctx.arg2::<usize>();

    let fd = ctx.fd_table.lock().get(fd).ok_or(-1isize)?;

    if !fd.can_write() {
        return Err(-1);
    }

    let file = fd.access();

    while !file.write_avaliable() {
        yield_now().await;
    }

    let buf = unsafe { core::slice::from_raw_parts(p_buf as *mut u8, len) };

    match ctx
        .borrow_page_table()
        .guard_slice(buf)
        .mustbe_user()
        .with_read()
    {
        Some(guard) => {
            let bytes_written = file.write(&guard);

            if let Some(file_meta) = file.metadata() {
                file_meta.set_offset(file_meta.offset() + bytes_written);
            }

            Ok(bytes_written as isize)
        }
        None => Err(-1),
    }
});

async_syscall!(sys_read_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let fd = ctx.fd_table.lock().get(fd).ok_or(-1isize)?;

    if !fd.can_read() {
        return Err(-1);
    }

    let file = fd.access();

    while !file.read_avaliable() {
        yield_now().await;
    }

    let p_buf = ctx.arg1::<*mut u8>();
    let len = ctx.arg2::<usize>();

    let buf = unsafe { core::slice::from_raw_parts_mut(p_buf, len) };

    match ctx
        .borrow_page_table()
        .guard_slice(buf)
        .mustbe_user()
        .mustbe_readable()
        .with_write()
    {
        Some(mut guard) => {
            let bytes_read = file.read(&mut guard);

            if let Some(file_meta) = file.metadata() {
                file_meta.set_offset(file_meta.offset() + bytes_read);
            }

            Ok(bytes_read as isize)
        }
        None => Err(-1),
    }
});

async_syscall!(sys_writev_async, ctx, {
    #[repr(C)]
    struct IoItem {
        p_data: *const u8,
        len: usize,
    }

    let fd = ctx.arg0::<usize>();
    let fd = ctx.fd_table.lock().get(fd).ok_or(-1isize)?;

    if !fd.can_write() {
        return Err(-1);
    }

    let file = fd.access();
    while !file.write_avaliable() {
        yield_now().await;
    }

    let iovec_base = ctx.arg1::<*const IoItem>();
    let len = ctx.arg2::<usize>();
    let io_vector = unsafe { core::slice::from_raw_parts(iovec_base, len) };

    match ctx
        .borrow_page_table()
        .guard_slice(io_vector)
        .mustbe_user()
        .with_read()
    {
        Some(vec_guard) => {
            let mut bytes_written = 0;

            for io in vec_guard.iter() {
                let data = unsafe { core::slice::from_raw_parts(io.p_data, io.len) };

                match ctx
                    .borrow_page_table()
                    .guard_slice(data)
                    .mustbe_user()
                    .with_read()
                {
                    Some(data_guard) => bytes_written += file.write(&data_guard),
                    None => continue,
                }
            }

            Ok(bytes_written as isize)
        }
        None => Err(-1isize),
    }
});

async_syscall!(sys_sendfile_async, ctx, {
    let (out_fd, in_fd) = {
        let fd_table = ctx.fd_table.lock();
        (
            fd_table.get(ctx.arg0::<usize>()).ok_or(-1isize)?, // out_fd
            fd_table.get(ctx.arg1::<usize>()).ok_or(-1isize)?, // in_fd
        )
    };

    if !out_fd.can_write() || !in_fd.can_read() {
        return Err(-1isize);
    }

    let in_file = in_fd.access();
    let in_meta = in_file.metadata();

    let poffset = ctx.arg2::<*const usize>();
    let size = ctx.arg3::<usize>();

    fn get_avaliable_size(
        file_meta: &Option<Arc<FileMetadata>>,
        size: usize,
    ) -> (Option<usize>, bool) {
        if let Some(file_meta) = file_meta {
            return match file_meta.inode().metadata() {
                Ok(inode_meta) => (
                    Some(usize::min(inode_meta.size - file_meta.offset(), size)),
                    true,
                ),
                Err(_) => (None, true),
            };
        }

        (None, false)
    }

    let (mut file_size, seekable) = get_avaliable_size(&in_meta, size);

    let out_file = out_fd.access();

    if seekable && !poffset.is_null() {
        let offset: usize = *ctx
            .borrow_page_table()
            .guard_ptr(poffset)
            .mustbe_user()
            .with_read()
            .ok_or(-1isize)?;

        in_meta.as_ref().unwrap().set_offset(offset);
    }

    let mut bytes_written = 0;
    while file_size.is_none() || file_size != Some(0) {
        let buf: MaybeUninit<[u8; 512]> = MaybeUninit::uninit();
        let mut buf: [u8; 512] = unsafe { core::mem::transmute::<_, _>(buf) };

        while !in_file.read_avaliable() {
            yield_now().await;
        }

        let bytes_read = in_file.read(&mut buf[..usize::min(512, file_size.unwrap_or(512))]);

        while !out_file.write_avaliable() {
            yield_now().await;
        }

        bytes_written += out_file.write(&buf[..bytes_read]);

        if let Some(old_size) = file_size {
            file_size = Some(old_size - bytes_read);
        }

        if seekable {
            let in_meta = in_meta.as_ref().unwrap();
            in_meta.set_offset(in_meta.offset() + bytes_read);
        }
    }
    Ok(bytes_written as isize)
});
