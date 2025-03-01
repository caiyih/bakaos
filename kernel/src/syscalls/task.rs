use core::str;

use abstractions::operations::IUsizeAlias;
use address::{IPageNum, IToPageNum, VirtualAddress};
use alloc::vec::Vec;
use constants::{ErrNo, SyscallError};
use filesystem_abstractions::DirectoryEntryType;
use log::debug;
use paging::{
    page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder, PageTable,
    PageTableEntryFlags,
};
use platform_abstractions::ISyscallContext;
use platform_specific::ITaskContext;
use tasks::{TaskCloneFlags, TaskStatus};
use timing::{TimeSpec, TimeVal};

use crate::{scheduling::spawn_task, timing::ITimer};

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};

pub struct ExitSyscall;

impl ISyncSyscallHandler for ExitSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let code = ctx.arg0::<isize>();

        ctx.exit_code
            .store(code as i32, core::sync::atomic::Ordering::Relaxed);
        *ctx.task_status.lock() = TaskStatus::Exited;

        debug!("Task {} exited with code {}", ctx.task_id.id(), code);
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
            .borrow_page_table()
            .guard_ptr(p_tms)
            .mustbe_user()
            .with_write()
        {
            Some(mut guard) => {
                // defined in <time.h>
                const CLOCKS_PER_SEC: f64 = 1000000.0;

                let timer_elapsed = ctx.timer.lock().elapsed().total_seconds();
                let kernel_elapsed = ctx.kernel_timer.lock().elapsed().total_seconds();

                guard.tms_utime = (timer_elapsed * CLOCKS_PER_SEC) as i64;
                guard.tms_stime = (kernel_elapsed * CLOCKS_PER_SEC) as i64;

                let children_timer_elapsed = ctx.children.lock().iter().fold(0f64, |acc, child| {
                    let child_timer = child.timer.lock().clone();
                    acc + child_timer.elapsed().total_microseconds()
                });

                let children_kernel_elapsed =
                    ctx.children.lock().iter().fold(0f64, |acc, child| {
                        let child_kernel_timer = child.kernel_timer.lock().clone();
                        acc + child_kernel_timer.elapsed().total_microseconds()
                    });

                guard.tms_cutime =
                    ((children_timer_elapsed - children_kernel_elapsed) * CLOCKS_PER_SEC) as i64;

                guard.tms_cstime = (children_kernel_elapsed * CLOCKS_PER_SEC) as i64;

                Ok(0)
            }
            None => SyscallError::BadAddress,
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

        let mut pcb = ctx.pcb.lock();

        let current_brk = pcb.brk_pos;

        if brk == 0 || brk == current_brk {
            return Ok(current_brk as isize);
        }

        if brk < current_brk {
            return SyscallError::OperationNotPermitted;
        }

        let memory_space = &mut pcb.memory_space;
        let brk_area = memory_space.brk_page_range();

        // new brk is in the same page, no need to allocate new pages
        // Only update brk position
        let brk_page_end = brk_area.end().start_addr::<VirtualAddress>().as_usize();
        if brk < brk_page_end {
            pcb.brk_pos = brk;
            return Ok(brk as isize);
        }

        let va = VirtualAddress::from_usize(brk);
        let vpn = va.to_ceil_page_num(); // end is exclusive

        match memory_space.increase_brk(vpn) {
            Ok(_) => {
                pcb.brk_pos = brk;
                Ok(brk as isize)
            }
            Err(reason) => {
                debug!("Failed to increase brk to {:#x}, reason: {}", brk, reason);
                SyscallError::OperationNotPermitted
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
            .borrow_page_table()
            .guard_ptr(tv)
            .mustbe_user()
            .with_write()
        {
            Some(mut guard) => {
                *guard = crate::timing::current_timeval();
                Ok(0)
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_gettimeofday"
    }
}

pub struct GetPidSyscall;

impl ISyncSyscallHandler for GetPidSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        Ok(ctx.pcb.lock().id as isize)
    }

    fn name(&self) -> &str {
        "sys_getpid"
    }
}

pub struct GetParentPidSyscall;

impl ISyncSyscallHandler for GetParentPidSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let parent_tcb = ctx.pcb.lock().parent.as_ref().and_then(|p| p.upgrade());

        Ok(parent_tcb.map(|p| p.pcb.lock().id).unwrap_or(1) as isize)
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

        let pcb = ctx.pcb.lock();
        let cwd = pcb.cwd.as_bytes();
        let len = cwd.len() + 1;

        debug_assert!(len > 0, "cwd remains uninitialized");

        if size < len {
            return SyscallError::NumericalResultOutOfRange;
        }

        let dst_slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };

        match ctx
            .borrow_page_table()
            .guard_slice(dst_slice)
            .mustbe_user()
            .with_write()
        {
            Some(mut guard) => {
                guard[..len - 1].copy_from_slice(cwd);
                guard[len - 1] = 0;
                Ok(buf as isize)
            }
            None => SyscallError::BadAddress,
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
        let _tls = ctx.arg3::<usize>();
        let pctid = ctx.arg4::<*mut usize>();

        // TODO: Implement thread fork
        let new_task = ctx.fork_process();
        let new_tid = new_task.task_id.id();

        ctx.children.lock().push(new_task.clone());

        debug!(
            "Forking task: {} from: {}, thread: {}",
            new_tid,
            ctx.task_id.id(),
            flags.contains(TaskCloneFlags::THREAD)
        );

        let new_trap_ctx = new_task.mut_trap_ctx();

        new_trap_ctx.set_syscall_return_value(0); // Child task's return value is 0

        if sp.as_usize() != 0 {
            new_trap_ctx.set_stack_top(sp.as_usize());
        }

        if flags.contains(TaskCloneFlags::PARENT_SETTID) {
            match ctx
                .borrow_page_table()
                .guard_ptr(ptid)
                .mustbe_user()
                .mustbe_readable()
                .with_write()
            {
                Some(mut guard) => *guard = new_tid,
                None => return SyscallError::BadAddress,
            }
        }

        if flags.contains(TaskCloneFlags::CHILD_SETTID) {
            let child_pt = new_task.borrow_page_table();

            if pctid.is_null() {
                return SyscallError::BadAddress;
            }

            // Copy through higher half address
            ctx.borrow_page_table().activated_copy_val_to_other(
                VirtualAddress::from_ptr(pctid),
                child_pt,
                &new_tid,
            );
        }

        // FIXME: figure out a way to do this under multiple arch
        // if flags.contains(TaskCloneFlags::SETTLS) {
        //     ctx.mut_trap_ctx().regs.tp = tls;
        // }

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
        fn guard_create_unsized_cstr_array(
            pt: &PageTable,
            mut ptr: *const *const u8,
        ) -> Option<Vec<&str>> {
            match pt
                .guard_unsized_cstr_array(ptr, 1024)
                .must_have(PageTableEntryFlags::User)
                .with(PageTableEntryFlags::Readable)
            {
                Some(_) => {
                    let mut array = Vec::new();
                    while !unsafe { ptr.read_volatile().is_null() } {
                        match pt
                            .guard_cstr(unsafe { *ptr }, 1024)
                            .must_have(PageTableEntryFlags::User)
                            .with(PageTableEntryFlags::Readable)
                        {
                            Some(str_guard) => unsafe {
                                let bytes = core::slice::from_raw_parts(*ptr, str_guard.len());
                                let str = core::str::from_utf8_unchecked(bytes);

                                array.push(str);
                            },
                            None => return None,
                        }

                        ptr = unsafe { ptr.add(1) };
                    }
                    Some(array)
                }
                None => None,
            }
        }

        let pathname = ctx.arg0::<*const u8>();

        let args = ctx.arg1::<*const *const u8>();
        let envp = ctx.arg2::<*const *const u8>();

        match ctx
            .borrow_page_table()
            .guard_cstr(pathname, 1024)
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(path_guard) => {
                let path = str::from_utf8(&path_guard).map_err(|_| ErrNo::InvalidArgument)?;
                let fullpath = {
                    let pcb = ctx.pcb.lock();
                    path::get_full_path(path, Some(&pcb.cwd))
                };

                match fullpath {
                    Some(fullpath) => {
                        let pt = ctx.borrow_page_table();

                        let args =
                            guard_create_unsized_cstr_array(pt, args).ok_or(ErrNo::BadAddress)?;
                        let envp =
                            guard_create_unsized_cstr_array(pt, envp).ok_or(ErrNo::BadAddress)?;

                        debug!(
                            "Task {} execve: '{}', args: {:?}, envp: {:?}",
                            ctx.task_id.id(),
                            path,
                            args,
                            envp
                        );

                        let file = filesystem_abstractions::global_open(&fullpath, None)
                            .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                        let bytes = file.readall().map_err(|_| ErrNo::OperationNotPermitted)?;

                        ctx.execve(&bytes, &fullpath, &args, &envp)
                            .map_err(|_| ErrNo::ExecFormatError)?;

                        unsafe {
                            *ctx.start_time.get().as_mut().unwrap().assume_init_mut() =
                                crate::timing::current_timespec();
                            ctx.kernel_timer.lock().start();
                            ctx.timer.lock().start();
                        }

                        Ok(0)
                    }
                    None => SyscallError::InvalidArgument,
                }
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_execve"
    }
}

pub struct ChdirSyscall;

impl ISyncSyscallHandler for ChdirSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let p_path = ctx.arg0::<*const u8>();

        match ctx
            .borrow_page_table()
            .guard_cstr(p_path, 1024)
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
        {
            Some(guard) => {
                let path = str::from_utf8(&guard).map_err(|_| ErrNo::InvalidArgument)?;

                let mut pcb = ctx.pcb.lock();
                match path::get_full_path(path, Some(&pcb.cwd)) {
                    Some(fullpath) => {
                        let processed_path = path::remove_relative_segments(&fullpath);
                        let inode = filesystem_abstractions::global_open(&processed_path, None)
                            .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                        let inode_metadata = inode.metadata();

                        match inode_metadata.entry_type {
                            DirectoryEntryType::Directory => {
                                pcb.cwd = processed_path;

                                Ok(0)
                            }
                            _ => SyscallError::NotADirectory,
                        }
                    }
                    None => SyscallError::InvalidArgument,
                }
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_chdir"
    }
}

pub struct ClockGetTimeSyscall;

impl ISyncSyscallHandler for ClockGetTimeSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        const CLOCK_REALTIME: usize = 0;
        const CLOCK_MONOTONIC: usize = 1;
        const CLOCK_PROCESS_CPUTIME_ID: usize = 2;

        let p_ts = ctx.arg1::<*mut TimeSpec>();

        match ctx
            .borrow_page_table()
            .guard_ptr(p_ts)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
        {
            Some(mut guard) => {
                match ctx.arg0::<usize>() {
                    CLOCK_REALTIME | CLOCK_MONOTONIC => {
                        *guard = crate::timing::current_timespec();
                        Ok(0)
                    }
                    CLOCK_PROCESS_CPUTIME_ID => {
                        let self_elapsed = ctx.timer.lock().elapsed().ticks();
                        let children_elapsed: i64 = ctx
                            .children
                            .lock()
                            .iter()
                            .map(|c| c.timer.lock().elapsed().ticks())
                            .sum();

                        *guard = TimeSpec::from_ticks(self_elapsed + children_elapsed, 10_000_000); // TimeSpan's freq

                        Ok(0)
                    }
                    _ => SyscallError::InvalidArgument,
                }
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_clock_gettime"
    }
}

pub struct ExitGroupSyscall;

impl ISyncSyscallHandler for ExitGroupSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let mut pcb = ctx.pcb.lock();
        let exit_code = ctx.arg0::<isize>();

        for task in pcb
            .tasks
            .values()
            .filter_map(|weak| weak.upgrade())
            .filter(|t| !t.is_exited())
        {
            task.exit_code
                .store(exit_code as i32, core::sync::atomic::Ordering::Relaxed);

            *task.task_status.lock() = TaskStatus::Exited;
        }

        pcb.status = TaskStatus::Exited;
        pcb.exit_code = exit_code as i32;

        debug!("Task group {} exited with code {}", pcb.id, exit_code);
        Ok(0)
    }

    fn name(&self) -> &str {
        "sys_exit_group"
    }
}

pub struct ResourceLimitSyscall;

impl ISyncSyscallHandler for ResourceLimitSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        const RLIMIT_NOFILE: usize = 7;

        #[repr(C)]
        #[allow(non_camel_case_types)]
        struct rlimit {
            rlim_cur: u64,
            rlim_max: u64,
        }

        let resource_id = ctx.arg1::<usize>();

        let p_new_limit = ctx.arg2::<*const rlimit>();
        let p_old_limit = ctx.arg3::<*mut rlimit>();

        let pt = ctx.borrow_page_table();

        let (new_limit, old_limit) = (
            pt.guard_ptr(p_new_limit).mustbe_user().with_read(),
            pt.guard_ptr(p_old_limit).mustbe_user().with_write(),
        );

        if resource_id == RLIMIT_NOFILE {
            let mut pcb = ctx.pcb.lock();

            if let Some(mut old_limit) = old_limit {
                old_limit.rlim_cur = pcb.fd_table.get_capacity() as u64;
                old_limit.rlim_max = old_limit.rlim_cur;
            }

            if let Some(new_limit) = new_limit {
                pcb.fd_table.set_capacity(new_limit.rlim_max as usize);
            }
        }

        Ok(0)
    }

    fn name(&self) -> &str {
        "sys_prlimit64"
    }
}

pub struct GetTaskIdSyscall;

impl ISyncSyscallHandler for GetTaskIdSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        Ok(ctx.task_id.id() as isize)
    }

    fn name(&self) -> &str {
        "sys_gettid"
    }
}
