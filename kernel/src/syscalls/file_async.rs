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
        Some(guard) => Ok(file.write(&guard) as isize),
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
        Some(mut guard) => Ok(file.read(&mut guard) as isize),
        None => Err(-1),
    }
});
