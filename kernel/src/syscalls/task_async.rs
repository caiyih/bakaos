use core::sync::atomic::Ordering;

use alloc::sync::Arc;
use constants::SyscallError;
use drivers::current_timespec;
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use platform_abstractions::ISyscallContext;
use threading::yield_now;
use timing::TimeSpec;

use crate::async_syscall;

async_syscall!(sys_nanosleep_async, ctx, {
    let req = ctx.arg0::<*const TimeSpec>();

    match ctx
        .borrow_page_table()
        .guard_ptr(req)
        .mustbe_user()
        .mustbe_readable()
        .with_write()
    {
        Some(guard) => {
            let start = current_timespec();
            let end = start + *guard;

            while current_timespec() < end {
                yield_now().await;
            }

            Ok(0)
        }
        None => SyscallError::BadAddress,
    }
});

// The logging code before the handler body requires ctx param, so we can't use a discard
async_syscall!(sys_sched_yield_async, ctx, {
    yield_now().await;

    // In the Linux implementation, sched_yield() always succeeds.
    Ok(0)
});

async_syscall!(sys_wait4_async, ctx, {
    let pid = ctx.arg0::<isize>();
    let nohang = (ctx.arg1::<i32>() & 1) == 1;

    loop {
        let exited_task = {
            let children = ctx.children.lock();

            if children.is_empty() {
                return SyscallError::NoChildProcesses;
            }

            match pid {
                -1 => children.iter().find(|c| c.is_exited()).cloned(),
                p if p > 0 => children
                    .iter()
                    .find(|c| c.task_id.id() == p as usize)
                    .cloned(),
                _ => unimplemented!(),
            }
        };

        match exited_task {
            Some(target_task) => {
                if !target_task.is_exited() {
                    if nohang {
                        return SyscallError::Success;
                    }

                    yield_now().await;
                    continue;
                }

                ctx.children
                    .lock()
                    .retain(|c| !Arc::ptr_eq(c, &target_task));

                let p_code = ctx.arg1::<*const i32>();
                if let Some(mut guard) = ctx
                    .borrow_page_table()
                    .guard_ptr(p_code)
                    .mustbe_user()
                    .mustbe_readable()
                    .with_write()
                {
                    *guard = (target_task.exit_code.load(Ordering::Relaxed) << 8) & 0xff00;
                }

                return Ok(target_task.task_id.id() as isize);
            }
            None if nohang => return SyscallError::Success,
            None => {
                // TODO: setup wakeup signal and wait.
                yield_now().await;
                continue;
            }
        }
    }
});
