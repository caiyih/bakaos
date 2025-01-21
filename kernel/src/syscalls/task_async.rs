use core::sync::atomic::Ordering;

use alloc::sync::Arc;
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use tasks::TaskControlBlock;
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

// The logging code before the handler body requires ctx param, so we can't use a discard
async_syscall!(sys_sched_yield_async, ctx, {
    yield_now().await;

    // In the Linux implementation, sched_yield() always succeeds.
    Ok(0)
});

async_syscall!(sys_wait4_async, ctx, {
    let pid = ctx.arg0::<isize>();
    let p_code = ctx.arg1::<usize>(); // pointer can not across await point, so we cast it later

    let target_child = ctx
        .children
        .lock()
        .iter()
        .find(|t| t.task_id.id() == pid as usize)
        .cloned();

    let exited_task: Arc<TaskControlBlock>;

    match target_child {
        Some(c) => {
            while !c.is_exited() {
                yield_now().await;
            }

            exited_task = c;
        }
        None => {
            if ctx.children.lock().is_empty() {
                return Err(-1);
            }

            loop {
                // Explicity limit the scope of the lock to prevent deadlock
                let exited_child = {
                    let children = ctx.children.lock();
                    children.iter().find(|t| t.is_exited()).cloned()
                };

                match exited_child {
                    Some(exited) => {
                        exited_task = exited;
                        break;
                    }
                    None => yield_now().await,
                }
            }
        }
    };

    if let Some(mut guard) = ctx
        .borrow_page_table()
        .guard_ptr(p_code as *const i32)
        .mustbe_user()
        .mustbe_readable()
        .with_write()
    {
        *guard = (exited_task.exit_code.load(Ordering::Relaxed) << 8) & 0xff00;
    }

    Ok(exited_task.task_id.id() as isize)
});
