use core::{str, sync::atomic::Ordering};

use abstractions::operations::IUsizeAlias;
use address::{IPageNum, IToPageNum, VirtualAddress};
use log::debug;
use paging::{
    page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder, PageTableEntryFlags,
};
use tasks::{TaskCloneFlags, TaskStatus};
use timing::TimeVal;

use crate::{scheduling::spawn_task, timing::ITimer};

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};

pub struct ExitSyscall;

impl ISyncSyscallHandler for ExitSyscall {
    fn handle(&self, ctx: &mut SyscallContext<'_>) -> SyscallResult {
        let code = ctx.arg0::<isize>();

        ctx.tcb
            .exit_code
            .store(code as i32, core::sync::atomic::Ordering::Relaxed);
        *ctx.tcb.task_status.lock() = TaskStatus::Exited;

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

        match ctx
            .tcb
            .borrow_page_table()
            .guard_ptr(p_tms)
            .mustbe_user()
            .with_write()
        {
            Some(mut guard) => {
                // defined in <time.h>
                const CLOCKS_PER_SEC: f64 = 1000000.0;

                let timer_elapsed = ctx.tcb.timer.lock().elapsed().total_seconds();
                let kernel_elapsed = ctx.tcb.kernel_timer.lock().elapsed().total_seconds();

                guard.tms_utime = ((timer_elapsed - kernel_elapsed) * CLOCKS_PER_SEC) as i64;
                guard.tms_stime = (kernel_elapsed * CLOCKS_PER_SEC) as i64;

                let children_timer_elapsed =
                    ctx.tcb.children.lock().iter().fold(0f64, |acc, child| {
                        let child_timer = child.timer.lock().clone();
                        acc + child_timer.elapsed().total_microseconds()
                    });

                let children_kernel_elapsed =
                    ctx.tcb.children.lock().iter().fold(0f64, |acc, child| {
                        let child_kernel_timer = child.kernel_timer.lock().clone();
                        acc + child_kernel_timer.elapsed().total_microseconds()
                    });

                guard.tms_cutime =
                    ((children_timer_elapsed - children_kernel_elapsed) * CLOCKS_PER_SEC) as i64;

                guard.tms_cstime = (children_kernel_elapsed * CLOCKS_PER_SEC) as i64;

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

        match ctx
            .tcb
            .borrow_page_table()
            .guard_ptr(tv)
            .mustbe_user()
            .with_write()
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

        match ctx
            .tcb
            .borrow_page_table()
            .guard_slice(dst_slice)
            .mustbe_user()
            .with_write()
        {
            Some(mut guard) => {
                guard.copy_from_slice(cwd);
                Ok(len as isize)
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_getcwd"
    }
}

pub struct CloneSyscall;

impl ISyncSyscallHandler for CloneSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let flags = ctx.arg0::<TaskCloneFlags>();
        let sp = ctx.arg1::<VirtualAddress>();
        let ptid = ctx.arg2::<*mut usize>();
        let tls = ctx.arg3::<usize>();
        let pctid = ctx.arg4::<*mut usize>();

        // TODO: Implement thread fork
        let new_task = ctx.tcb.fork_process();
        let new_tid = new_task.task_id.id();

        ctx.tcb.children.lock().push(new_task.clone());

        debug!("Forking task: {} from: {}", new_tid, ctx.tcb.task_id.id());

        let new_trap_ctx = new_task.mut_trap_ctx();

        new_trap_ctx.regs.a0 = 0; // Child task's return value is 0

        if sp.as_usize() != 0 {
            new_trap_ctx.regs.sp = sp;
        }

        if flags.contains(TaskCloneFlags::PARENT_SETTID) {
            match ctx
                .tcb
                .borrow_page_table()
                .guard_ptr(ptid)
                .mustbe_user()
                .mustbe_readable()
                .with_write()
            {
                Some(mut guard) => *guard = new_tid,
                None => return Err(-1),
            }
        }

        if flags.contains(TaskCloneFlags::CHILD_SETTID) {
            let child_pt = new_task.borrow_page_table();

            if pctid.is_null() {
                return Err(-1);
            }

            // Copy through higher half address
            ctx.tcb.borrow_page_table().activated_copy_val_to_other(
                VirtualAddress::from_ptr(pctid),
                &child_pt,
                &new_tid,
            );
        }

        if flags.contains(TaskCloneFlags::SETTLS) {
            ctx.tcb.mut_trap_ctx().regs.tp = tls;
        }

        // TODO: Set clear tid address to pctid

        spawn_task(new_task);

        Ok(new_tid as isize)
    }

    fn name(&self) -> &str {
        "sys_clone"
    }
}

pub struct ExecveSyscall;

impl ISyncSyscallHandler for ExecveSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let pathname = ctx.arg0::<*const u8>();

        let _args = ctx.arg1::<*const *const u8>();
        let _envp = ctx.arg2::<*const *const u8>();

        match ctx
            .tcb
            .borrow_page_table()
            .guard_cstr(pathname, 1024)
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let path = str::from_utf8(&guard).map_err(|_| -1isize)?;
                debug!("Task {} execve: '{}'", ctx.tcb.task_id.id(), path);

                match path::get_full_path(
                    path,
                    Some(unsafe { ctx.tcb.cwd.get().as_ref().unwrap() }),
                ) {
                    Some(fullpath) => {
                        let file = filesystem::root_filesystem()
                            .lookup(&fullpath)
                            .map_err(|_| -1isize)?;

                        let bytes = file.readall().map_err(|_| -1isize)?;

                        // TODO: handle args and envp
                        ctx.tcb.execve(&bytes, &[], &[]).map_err(|_| -1isize)?;

                        unsafe {
                            *ctx.tcb.start_time.get().as_mut().unwrap().assume_init_mut() =
                                crate::timing::current_timespec();
                            ctx.tcb.kernel_timer.lock().start();
                            ctx.tcb.timer.lock().start();
                        }

                        Ok(0)
                    }
                    None => Err(-1),
                }
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_execve"
    }
}
