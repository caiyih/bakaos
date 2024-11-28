use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use threading::yield_now;

use crate::async_syscall;

async_syscall!(sys_read_async, ctx, {
    let fd = ctx.arg0::<usize>();
    let fd = ctx.tcb.fd_table.lock().get(fd).ok_or(-1isize)?;

    if !fd.can_read() {
        return Err(-1);
    }

    let file = fd.access().ok_or(-1isize)?; // check if file is closed

    if !file.read_avaliable() {
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
        Some(mut guard) => Ok(file.read(guard.as_mut()) as isize),
        None => Err(-1),
    }
});
