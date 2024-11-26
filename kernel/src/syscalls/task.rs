use log::debug;
use paging::{IWithPageGuardBuilder, PageTableEntryFlags};
use tasks::TaskStatus;

use crate::timing::ITimer;

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};

pub struct ExitSyscall;

impl ISyncSyscallHandler for ExitSyscall {
    fn handle(&self, ctx: &mut SyscallContext<'_>) -> SyscallResult {
        let code = ctx.arg0::<isize>();

        *ctx.tcb.task_status.lock() = TaskStatus::Exited;
        ctx.tcb
            .exit_code
            .store(code as i32, core::sync::atomic::Ordering::Relaxed);

        debug!("Task {} exited with code {}", ctx.tcb.task_id.id(), code);
        Ok(0)
    }

    fn name(&self) -> &str {
        "sys_exit"
    }
}

#[repr(C)]
struct Tms {
    tms_utime: i64,
    tms_stime: i64,
    tms_cutime: i64,
    tms_cstime: i64,
}

pub struct TimesSyscall;

impl ISyncSyscallHandler for TimesSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let p_tms = ctx.arg0::<*mut Tms>();

        let memory_space = ctx.tcb.memory_space.lock();
        match memory_space
            .page_table()
            .guard_ptr(p_tms)
            .must_have(PageTableEntryFlags::User)
            .with(PageTableEntryFlags::Writable)
        {
            Some(mut guard) => {
                let user_timer = ctx.tcb.timer.lock().clone();
                let kernel_timer: tasks::UserTaskTimer = ctx.tcb.kernel_timer.lock().clone();

                guard.tms_utime = user_timer.elapsed().total_microseconds() as i64;
                guard.tms_stime = kernel_timer.elapsed().total_microseconds() as i64;
                // TODO: calculate tms_cutime and tms_cstime

                Ok(0)
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_times"
    }
}
