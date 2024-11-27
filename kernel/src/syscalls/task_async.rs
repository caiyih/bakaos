use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use threading::yield_now;
use timing::TimeSpec;

use crate::async_syscall;

use super::{SyscallContext, SyscallResult};

async_syscall!(sys_nanosleep_async, ctx, {
    let req = ctx.arg0::<*const TimeSpec>();

    match ctx
        .tcb
        .borrow_page_table()
        .guard_ptr(req)
        .mustbe_user()
        .mustbe_readable()
        .with_write()
    {
        Some(guard) => {
            let start = crate::timing::current_timespec();
            let end = start + *guard;

            while crate::timing::current_timespec() < end {
                yield_now().await;
            }

            Ok(0)
        }
        None => Err(-1),
    }
});
