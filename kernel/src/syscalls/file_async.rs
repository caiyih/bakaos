use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use threading::yield_now;

use crate::async_syscall;

async_syscall!(sys_write_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let p_buf = ctx.arg1::<usize>();
    let len = ctx.arg2::<usize>();

    let fd = ctx.tcb.fd_table.lock().get(fd).ok_or(-1isize)?;

    if !fd.can_write() {
        return Err(-1);
    }

    let file = fd.access();

    while !file.write_avaliable() {
        yield_now().await;
    }

    let buf = unsafe { core::slice::from_raw_parts(p_buf as *mut u8, len) };

    match ctx
        .tcb
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
    let fd = ctx.tcb.fd_table.lock().get(fd).ok_or(-1isize)?;

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
        .tcb
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
    let fd = ctx.tcb.fd_table.lock().get(fd).ok_or(-1isize)?;

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
        .tcb
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
                    .tcb
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
