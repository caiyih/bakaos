use constants::ErrNo;
use drivers::current_timespec;
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use platform_abstractions::ISyscallContext;

use crate::{dmesg::read_dmesg, memory, scheduling};

use super::{ISyncSyscallHandler, SyscallContext, SyscallResult};

pub struct GetRandomSyscall;

impl ISyncSyscallHandler for GetRandomSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let p_buf = ctx.arg0::<*mut u8>();
        let len = ctx.arg1::<usize>();

        let slice = unsafe { core::slice::from_raw_parts_mut(p_buf, len) };

        let mut guard = ctx
            .borrow_page_table()
            .guard_slice(slice)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
            .ok_or(ErrNo::BadAddress)?;

        rng::global_fill_safe(&mut guard);

        Ok(len as isize)
    }

    fn name(&self) -> &str {
        "sys_getrandom"
    }
}

pub struct SystemInfoSyscall;

impl ISyncSyscallHandler for SystemInfoSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        #[repr(C)]
        struct SysInfo {
            /// Seconds since boot
            pub uptime: isize,
            pub loads: [usize; 3],
            pub total_ram: usize,
            pub free_ram: usize,
            pub shared_ram: usize,
            pub buffer_ram: usize,
            pub total_swap: usize,
            pub free_swap: usize,
            pub procs: u16,
            __pad1: [u8; 6],
            pub total_high: usize,
            pub free_high: usize,
            pub mem_uint: u32,
        }

        let p_info = ctx.arg0::<*mut SysInfo>();

        let mut guard = ctx
            .borrow_page_table()
            .guard_ptr(p_info)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
            .ok_or(ErrNo::BadAddress)?;

        guard.uptime = current_timespec().total_seconds() as isize;

        let (avaliable, _, total) = allocation::allocation_statistics();
        let (heap_requested, _, heap_total) = memory::heap_statistics();

        guard.total_ram = total;
        guard.free_ram = avaliable;
        guard.shared_ram = 0;
        guard.buffer_ram = 0;
        guard.total_swap = 0;
        guard.free_swap = 0;
        guard.procs = scheduling::task_count() as u16;
        guard.total_high = heap_total;
        guard.free_high = heap_total - heap_requested;
        guard.mem_uint = 1;

        Ok(0)
    }

    fn name(&self) -> &str {
        "sys_sysinfo"
    }
}

pub struct ShutdownSyscall;

impl ISyncSyscallHandler for ShutdownSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        log::warn!("Shutdown syscall from task: {}", ctx.task_id.id());

        platform_abstractions::machine_shutdown(false)
    }

    fn name(&self) -> &str {
        "sys_shutdown"
    }
}

pub struct SystemLogSyscall;

impl ISyncSyscallHandler for SystemLogSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let cmd = ctx.arg0::<i32>();
        let p_buf = ctx.arg1::<*mut u8>();
        let len = ctx.arg2::<usize>();

        // read all
        if cmd != 3 {
            return Ok(0);
        }

        let slice = unsafe { core::slice::from_raw_parts_mut(p_buf, len) };

        let mut guard = ctx
            .borrow_page_table()
            .guard_slice(slice)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
            .ok_or(ErrNo::BadAddress)?;

        Ok(read_dmesg(&mut guard) as isize)
    }

    fn name(&self) -> &str {
        "sys_syslog"
    }
}
