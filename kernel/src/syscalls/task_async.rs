use paging::{IWithPageGuardBuilder, PageTableEntryFlags};
use threading::yield_now;
use timing::TimeSpec;

use super::{SyscallContext, SyscallResult};

pub async fn sys_nanosleep_async(ctx: &mut SyscallContext<'_>) -> SyscallResult {
    let req = ctx.arg0::<*const TimeSpec>();

    match ctx
        .tcb
        .borrow_page_table()
        .guard_ptr(req)
        .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        .with(PageTableEntryFlags::Writable)
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
}
