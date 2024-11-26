use core::sync::atomic::Ordering;

use abstractions::operations::IUsizeAlias;
use address::{IPageNum, IToPageNum, VirtualAddress};
use log::debug;
use paging::{IWithPageGuardBuilder, PageTableEntryFlags};
use tasks::TaskStatus;
use timing::TimeVal;

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

pub struct BrkSyscall;

impl ISyncSyscallHandler for BrkSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let brk = ctx.arg0::<usize>();

        let current_brk = ctx.tcb.brk_pos.load(Ordering::Relaxed);

        if brk == 0 || brk == current_brk {
            return Ok(current_brk as isize);
        }

        if brk < current_brk {
            return Err(-1);
        }

        let mut memory_space = ctx.tcb.memory_space.lock();
        let brk_area = memory_space.brk_page_range();

        // new brk is in the same page, no need to allocate new pages
        // Only update brk position
        let brk_page_end = brk_area.end().start_addr::<VirtualAddress>().as_usize();
        if brk < brk_page_end {
            ctx.tcb.brk_pos.store(brk, Ordering::Relaxed);
            return Ok(brk as isize);
        }

        let va = VirtualAddress::from_usize(brk);
        let vpn = va.to_ceil_page_num(); // end is exclusive

        match memory_space.increase_brk(vpn) {
            Ok(_) => {
                ctx.tcb.brk_pos.store(brk, Ordering::Relaxed);
                Ok(brk as isize)
            }
            Err(reason) => {
                debug!("Failed to increase brk to {:#x}, reason: {}", brk, reason);
                Err(-1)
            }
        }
    }

    fn name(&self) -> &str {
        "sys_brk"
    }
}

pub struct GetTimeOfDaySyscall;

impl ISyncSyscallHandler for GetTimeOfDaySyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let tv = ctx.arg0::<*mut TimeVal>();

        let memory_space = ctx.tcb.memory_space.lock();
        match memory_space
            .page_table()
            .guard_ptr(tv)
            .must_have(PageTableEntryFlags::User)
            .with(PageTableEntryFlags::Writable)
        {
            Some(mut guard) => {
                *guard = crate::timing::current_timeval();
                Ok(0)
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_gettimeofday"
    }
}

pub struct GetPidSyscall;

impl ISyncSyscallHandler for GetPidSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        Ok(ctx.tcb.task_id.id() as isize)
    }

    fn name(&self) -> &str {
        "sys_getpid"
    }
}

pub struct GetParentPidSyscall;

impl ISyncSyscallHandler for GetParentPidSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let parent = ctx.tcb.parent.as_ref().map(|p| p.upgrade().unwrap());
        Ok(parent.map(|p| p.task_id.id()).unwrap_or(1) as isize)
    }

    fn name(&self) -> &str {
        "sys_getppid"
    }
}

pub struct GetCwdSyscall;

impl ISyncSyscallHandler for GetCwdSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let buf = ctx.arg0::<*mut u8>();
        let size = ctx.arg1::<usize>();

        let cwd = unsafe { ctx.tcb.cwd.get().as_ref().unwrap().as_bytes() };
        let len = cwd.len();

        debug_assert!(len > 0, "cwd remains uninitialized");

        if size < len {
            return Err(-1);
        }

        let dst_slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };

        let memory_space = ctx.tcb.memory_space.lock();
        match memory_space
            .page_table()
            .guard_slice(dst_slice)
            .must_have(PageTableEntryFlags::User)
            .with(PageTableEntryFlags::Writable)
        {
            Some(mut guard) => {
                guard.copy_from_slice(&cwd);
                Ok(len as isize)
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_getcwd"
    }
}
