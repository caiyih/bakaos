use core::{ops::DerefMut, str};

use abstractions::operations::IUsizeAlias;
use address::{IAddressBase, IPageNum, IToPageNum, VirtualAddress};
use alloc::vec::Vec;
use constants::{ErrNo, SyscallError};
use drivers::{current_timespec, current_timeval, ITimer};
use filesystem_abstractions::DirectoryEntryType;
use log::debug;
use page_table::GenericMappingFlags;
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder, PageTable};
use platform_specific::{ISyscallContext, ISyscallContextMut, ITaskContext};
use tasks::{ProcessControlBlock, SyscallContext, TaskCloneFlags, TaskControlBlock, TaskStatus};
use timing::{TimeSpec, TimeVal};

use crate::scheduling::{self, spawn_task};

use super::{ISyncSyscallHandler, SyscallResult};

pub struct ExitSyscall;

impl ISyncSyscallHandler for ExitSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let code = ctx.arg0::<isize>();

        ctx.exit_code
            .store(code as i32, core::sync::atomic::Ordering::Relaxed);
        *ctx.task_status.lock() = TaskStatus::Exited;

        debug!("Task {} exited with code {}", ctx.task_id.id(), code);

        let mut pcb = ctx.pcb.lock();

        if pcb
            .tasks
            .iter()
            .filter_map(|(_, w)| w.upgrade())
            .all(|t| t.is_exited())
        {
            pcb.status = TaskStatus::Exited;
            pcb.exit_code = code as i32;

            debug!(
                "Process {} exited with code {}, exiting all its children",
                pcb.id, code
            );
        }

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
        let brk_page_end = brk_area.end().start_addr().as_usize();
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
                debug!("Failed to increase brk to {brk:#x}, reason: {reason}");
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
                *guard = current_timeval();
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

        match ctx
            .borrow_page_table()
            .guard_slice(buf, len)
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

        let ptid: *mut usize;
        let tls: usize;
        let pctid: *mut usize;

        #[cfg(target_arch = "riscv64")]
        {
            ptid = ctx.arg2::<*mut usize>();
            tls = ctx.arg3::<usize>();
            pctid = ctx.arg4::<*mut usize>();
        }

        #[cfg(target_arch = "loongarch64")]
        {
            ptid = ctx.arg2::<*mut usize>();
            pctid = ctx.arg3::<*mut usize>();
            tls = ctx.arg4::<usize>();
        }

        // TODO: Implement thread fork
        let is_thread = flags.contains(TaskCloneFlags::THREAD);

        let new_task = if is_thread {
            ctx.fork_thread()
        } else {
            ctx.fork_process(flags.contains(TaskCloneFlags::VM))
        };

        ctx.children.lock().push(new_task.clone());

        let new_tid = new_task.task_id.id();

        debug!(
            "Forking task: {} from: {}, flags: {:?}, tls: {:#x}, pctid: {:?}",
            new_tid,
            ctx.task_id.id(),
            flags,
            tls,
            pctid
        );

        let mut forked_syscall_ctx = new_task.to_syscall_context();

        forked_syscall_ctx.set_return_value(0); // Child task's return value is 0

        if !sp.is_null() {
            unsafe {
                forked_syscall_ctx
                    .mut_trap_ctx()
                    .set_stack_top(sp.as_usize())
            };
        }

        if flags.contains(TaskCloneFlags::PARENT_SETTID) {
            if let Some(mut guard) = ctx
                .borrow_page_table()
                .guard_ptr(ptid)
                .mustbe_user()
                .mustbe_readable()
                .with_write()
            {
                *guard = new_tid;
            }
        }

        if flags.contains(TaskCloneFlags::CHILD_SETTID) {
            let child_pt = new_task.borrow_page_table();

            if !pctid.is_null() {
                // Copy through higher half address
                ctx.borrow_page_table().activated_copy_val_to_other(
                    VirtualAddress::from_ptr(pctid),
                    child_pt,
                    &new_tid,
                );
            }
        }

        // FIXME: figure out a way to do this under multiple arch
        if flags.contains(TaskCloneFlags::SETTLS) {
            unsafe { forked_syscall_ctx.mut_trap_ctx().regs.tp = tls };
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
        fn guard_create_unsized_cstr_array(
            pt: &PageTable,
            mut ptr: *const *const u8,
        ) -> Option<Vec<&str>> {
            match pt
                .guard_unsized_cstr_array(ptr, 1024)
                .must_have(GenericMappingFlags::User)
                .with(GenericMappingFlags::Readable)
            {
                Some(_) => {
                    let mut array = Vec::new();
                    while !unsafe { ptr.read_volatile().is_null() } {
                        match pt
                            .guard_cstr(unsafe { *ptr }, 1024)
                            .must_have(GenericMappingFlags::User)
                            .with(GenericMappingFlags::Readable)
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
            .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable)
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

                        let file_inode = filesystem_abstractions::global_open(&fullpath, None)
                            .map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

                        ctx.execve(&file_inode, &fullpath, &args, &envp)
                            .map_err(|_| ErrNo::ExecFormatError)?;

                        unsafe {
                            *ctx.start_time.get().as_mut().unwrap().assume_init_mut() =
                                current_timespec();
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
            .must_have(GenericMappingFlags::User | GenericMappingFlags::Readable)
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
                        *guard = current_timespec();
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
        fn recursive_exit(task: &TaskControlBlock, exit_code: i32) {
            let mut pcb = task.pcb.lock();

            pcb.status = TaskStatus::Exited;
            pcb.exit_code = exit_code;

            let pgid = pcb.id;

            let children = pcb
                .tasks
                .values()
                .filter_map(|weak| weak.upgrade())
                .filter(|t| !t.is_exited())
                .collect::<Vec<_>>();

            drop(pcb);

            for child in children.into_iter() {
                child
                    .exit_code
                    .store(exit_code, core::sync::atomic::Ordering::Relaxed);

                *child.task_status.lock() = TaskStatus::Exited;

                // Process group leader
                // TODO: send signal instead of killing them directly
                if child.task_id.id() == pgid {
                    let mut child_pcb = child.pcb.lock();

                    for child_process_task in child_pcb
                        .tasks
                        .values()
                        .filter_map(|weak| weak.upgrade())
                        .filter(|t| !t.is_exited())
                    {
                        child
                            .exit_code
                            .store(exit_code, core::sync::atomic::Ordering::Relaxed);

                        *child_process_task.task_status.lock() = TaskStatus::Exited;
                    }

                    child_pcb.exit_code = exit_code;
                    child_pcb.status = TaskStatus::Exited;
                }
            }
        }

        let exit_code = ctx.arg0::<i32>();

        recursive_exit(ctx, exit_code);

        let pcb = ctx.pcb.lock();

        debug!("Task group {} exited with code {}", pcb.id, exit_code);

        if scheduling::task_count() > 1 // The initproc still exists
            && pcb.is_initproc.load(core::sync::atomic::Ordering::Relaxed)
        {
            log::warn!(
                "Shutting down the kernel due to initproc exit. Remaining tasks: {}",
                scheduling::task_count()
            );

            unsafe {
                platform_abstractions::machine_shutdown(cfg!(debug_assertions));
            }
        }

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

pub struct GetResUsageSyscall;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct ResUsage {
    utime: TimeVal,
    stime: TimeVal,
    maxrss: usize,
    ixrss: usize,
    idrss: usize,
    isrss: usize,
    minflt: usize,
    majflt: usize,
    nswap: usize,
    inblock: usize,
    oublock: usize,
    msgsnd: usize,
    msgrcv: usize,
    nsignals: usize,
    nvcsw: usize,
    nivcsw: usize,
}

fn task_time(task: &TaskControlBlock) -> (TimeVal, TimeVal) {
    // defined in <time.h>
    const CLOCKS_PER_SEC: f64 = 1000000.0;

    let timer_elapsed = task.timer.lock().elapsed().total_seconds();
    let kernel_elapsed = task.kernel_timer.lock().elapsed().total_seconds();

    (
        TimeVal::from_ticks(
            (timer_elapsed * CLOCKS_PER_SEC) as i64,
            CLOCKS_PER_SEC as u64,
        ),
        TimeVal::from_ticks(
            (kernel_elapsed * CLOCKS_PER_SEC) as i64,
            CLOCKS_PER_SEC as u64,
        ),
    )
}

fn process_time(process: &ProcessControlBlock) -> (TimeVal, TimeVal) {
    process
        .tasks
        .iter()
        .filter_map(|(_, w)| w.upgrade())
        .map(|t| task_time(&t))
        .fold((TimeVal::zero(), TimeVal::zero()), |a, b| {
            (a.0 + b.0, a.1 + b.1)
        })
}

fn children_time(task: &TaskControlBlock) -> (TimeVal, TimeVal) {
    task.children
        .lock()
        .iter()
        .map(|c| task_time(c))
        .fold((TimeVal::zero(), TimeVal::zero()), |a, b| {
            (a.0 + b.0, a.1 + b.1)
        })
}

impl ISyncSyscallHandler for GetResUsageSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        pub const RUSAGE_SELF: i32 = 0;
        pub const RUSAGE_CHILDREN: i32 = -1;
        pub const RUSAGE_THREAD: i32 = 1;

        let target = ctx.arg0::<i32>();
        let rusage_ptr = ctx.arg1::<VirtualAddress>();

        match ctx
            .borrow_page_table()
            .guard_ptr(rusage_ptr.as_ptr::<ResUsage>())
            .mustbe_user()
            .mustbe_readable()
            .with_write()
            .as_mut()
        {
            Some(rusage) => {
                let mut r = ResUsage::default();

                let (utime, stime) = match target {
                    RUSAGE_THREAD => task_time(ctx),
                    RUSAGE_SELF => process_time(&ctx.pcb.lock()),
                    RUSAGE_CHILDREN => children_time(ctx),
                    _ => return SyscallError::InvalidArgument,
                };

                r.utime = utime;
                r.stime = stime;

                *rusage.deref_mut() = r;

                SyscallError::Success
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_getrusage"
    }
}
