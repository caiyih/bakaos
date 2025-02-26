use core::intrinsics::atomic_load_acquire;

use address::VirtualAddress;
use alloc::sync::Arc;
use constants::{ErrNo, SyscallError};
use paging::IWithPageGuardBuilder;
use tasks::TaskControlBlock;
use threading::yield_now;
use timing::TimeSpec;

use crate::{async_syscall, timing::current_timespec};

async fn futex_wait(
    tcb: &Arc<TaskControlBlock>,
    uaddr: VirtualAddress,
    val: u32,
    timeout: Option<TimeSpec>,
) {
    tcb.pcb
        .lock()
        .futex_queue
        .enqueue(uaddr, tcb.task_id.id(), tcb.waker());

    let end_time = timeout.map(|t| t + current_timespec());

    loop {
        {
            let ptr = unsafe { uaddr.as_ptr::<u32>() };

            // prevent modification to page table, is this enough?
            let pcb = tcb.pcb.lock();

            if pcb
                .memory_space
                .page_table()
                .guard_ptr(ptr)
                .mustbe_user()
                .with_read()
                .is_some_and(|_| unsafe { atomic_load_acquire(ptr) != val })
            {
                break;
            }
        }

        if let Some(end_time) = end_time {
            if current_timespec() >= end_time {
                break;
            }
        }

        yield_now().await;
    }

    tcb.pcb
        .lock()
        .futex_queue
        .notify_woken(uaddr, tcb.task_id.id());
}

async_syscall!(sys_futex_async, ctx, {
    const FUTEX_OP_WAIT: i32 = 0;
    const FUTEX_OP_WAKE: i32 = 1;
    const FUTEX_OP_REQUEUE: i32 = 3;
    const FUTEX_OP_CMP_REQUEUE: i32 = 4;

    let futex_op = ctx.arg1::<i32>();

    match futex_op {
        FUTEX_OP_WAIT => {
            let val = ctx.arg2::<u32>();
            let uaddr = ctx.arg0::<VirtualAddress>();

            // validate uaddr
            ctx.borrow_page_table()
                .guard_ptr(unsafe { uaddr.as_ptr::<u32>() })
                .mustbe_user()
                .with_read()
                .ok_or(ErrNo::BadAddress)?;

            let p_timeout = ctx.arg3::<*const TimeSpec>();
            let timeout = ctx
                .borrow_page_table()
                .guard_ptr(p_timeout)
                .mustbe_user()
                .with_read()
                .map(|g| *g);

            if unsafe { atomic_load_acquire(uaddr.as_ptr::<u32>()) } != val {
                return SyscallError::ResourceTemporarilyUnavailable;
            }

            // Split to make state machine smaller
            futex_wait(ctx, uaddr, val, timeout).await;
        }
        FUTEX_OP_WAKE => {
            return Ok(ctx.pcb.lock().futex_queue.wake(
                ctx.arg0::<VirtualAddress>(), // uaddr
                ctx.arg2::<usize>(),          // nval
            ) as isize);
        }
        FUTEX_OP_REQUEUE | FUTEX_OP_CMP_REQUEUE => {
            if futex_op == FUTEX_OP_CMP_REQUEUE {
                let val = ctx.arg2::<u32>();
                let uaddr = ctx.arg0::<VirtualAddress>();

                if unsafe { atomic_load_acquire(uaddr.as_ptr::<u32>()) } != val {
                    return SyscallError::ResourceTemporarilyUnavailable;
                }
            }

            return Ok(ctx.pcb.lock().futex_queue.requeue(
                ctx.arg0::<VirtualAddress>(), // prev_addr
                ctx.arg4::<VirtualAddress>(), // new_addr
                ctx.arg2::<usize>(),          // n_wake
                ctx.arg3::<usize>(),          // n_requeue
            ) as isize);
        }
        _ => (),
    }

    Ok(0)
});
