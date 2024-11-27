#![allow(clippy::await_holding_lock)] // we manually dropped mutex guard before await

use core::sync::atomic::Ordering;

use alloc::sync::Arc;
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use tasks::TaskControlBlock;
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

// The logging code before the handler body requires ctx param, so we can't use a discard
async_syscall!(sys_sched_yield_async, ctx, {
    yield_now().await;

    // In the Linux implementation, sched_yield() always succeeds.
    Ok(0)
});

async_syscall!(sys_wait4_async, ctx, {
    let pid = ctx.arg0::<isize>();
    let p_code = ctx.arg1::<usize>();

    let target_child = ctx
        .tcb
        .children
        .lock()
        .iter()
        .find(|t| t.task_id.id() == pid as usize)
        .cloned();

    let exited_task: Arc<TaskControlBlock>;

    match target_child {
        Some(c) => {
            if !c.is_exited() {
                yield_now().await;
            }

            exited_task = c;
        }
        None => {
            if ctx.tcb.children.lock().is_empty() {
                return Err(-1);
            }

            loop {
                let child = ctx.tcb.children.lock();

                match child.iter().find(|t| t.is_exited()) {
                    Some(founded) => {
                        exited_task = founded.clone();
                        break;
                    }
                    None => {
                        drop(child); // prevent dead lock
                        yield_now().await
                    }
                }
            }
        }
    };

    let exit_code = exited_task.exit_code.load(Ordering::Relaxed);
    let exit_code = (exit_code << 8) & 0xff00;

    if let Some(mut guard) = ctx
        .tcb
        .borrow_page_table()
        .guard_ptr(p_code as *const i32)
        .mustbe_user()
        .mustbe_readable()
        .with_write()
    {
        *guard = exit_code;
    }

    Ok(exited_task.task_id.id() as isize)
});
