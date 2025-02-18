use core::ops::Deref;

use alloc::{format, sync::Arc};
use constants::SyscallError;
use file::{
    CloseSyscall, Dup3Syscall, DupSyscall, FileTruncateSyscall, GetDents64Syscall, LinkAtSyscall,
    LongSeekSyscall, MkdirAtSyscall, MmapSyscall, MountSyscall, MunmapSyscall, NewFstatSyscall,
    NewFstatatSyscall, OpenAtSyscall, Pipe2Syscall, ReadLinkAtSyscall, SymbolLinkAtSyscall,
    UmountSyscall, UnlinkAtSyscall,
};
use file_async::{
    sys_read_async, sys_readv_async, sys_sendfile_async, sys_write_async, sys_writev_async,
};
use futex_async::sys_futex_async;
use io_multiplexing::{sys_ppoll_async, sys_pselect6_async};
use paging::{page_table::IOptionalPageGuardBuilderExtension, IWithPageGuardBuilder};
use shm::{SharedMemoryAttachSyscall, SharedMemoryGetSyscall};
use system::{GetRandomSyscall, ShutdownSyscall, SystemInfoSyscall, SystemLogSyscall};
use task::{
    BrkSyscall, ChdirSyscall, ClockGetTimeSyscall, CloneSyscall, ExecveSyscall, ExitGroupSyscall,
    ExitSyscall, GetCwdSyscall, GetParentPidSyscall, GetPidSyscall, GetTaskIdSyscall,
    GetTimeOfDaySyscall, ResourceLimitSyscall, TimesSyscall,
};
use task_async::{sys_nanosleep_async, sys_sched_yield_async, sys_wait4_async};
use tasks::TaskControlBlock;

use syscall_id::*;

mod file;
mod file_async;
mod futex_async;
mod io_multiplexing;
mod shm;
mod syscall_id;
mod system;
mod task;
mod task_async;

pub trait ISyscallResult {
    fn to_ret(self) -> isize;
}

pub type SyscallResult = Result<isize, isize>;

impl ISyscallResult for SyscallResult {
    fn to_ret(self) -> isize {
        match self {
            Ok(val) => val,
            Err(err) => err,
        }
    }
}

pub struct SyscallDispatcher;

impl SyscallDispatcher {
    pub fn dispatch(
        tcb: &Arc<TaskControlBlock>,
        syscall_id: usize,
    ) -> Option<(SyscallContext, &'static dyn ISyncSyscallHandler)> {
        let handler = Self::translate_id(syscall_id)?;

        Some((SyscallContext::new(tcb), handler))
    }

    fn translate_id(id: usize) -> Option<&'static dyn ISyncSyscallHandler> {
        match id {
            SYSCALL_ID_GETCWD => Some(&GetCwdSyscall),
            SYSCALL_ID_EXIT => Some(&ExitSyscall),
            SYSCALL_ID_EXIT_GROUP => Some(&ExitGroupSyscall),
            SYSCALL_ID_TIMES => Some(&TimesSyscall),
            SYSCALL_ID_UNAME => Some(&UnameSyscall),
            SYSCALL_ID_GETTIMEOFDAY => Some(&GetTimeOfDaySyscall),
            SYSCALL_ID_GETPPID => Some(&GetParentPidSyscall),
            SYSCALL_ID_GETPID => Some(&GetPidSyscall),
            SYSCALL_ID_BRK => Some(&BrkSyscall),
            SYSCALL_ID_CLONE => Some(&CloneSyscall),
            SYSCALL_ID_EXECVE => Some(&ExecveSyscall),
            SYSCALL_ID_PIPE2 => Some(&Pipe2Syscall),
            SYSCALL_ID_OPENAT => Some(&OpenAtSyscall),
            SYSCALL_ID_CLOSE => Some(&CloseSyscall),
            SYSCALL_ID_DUP => Some(&DupSyscall),
            SYSCALL_ID_DUP3 => Some(&Dup3Syscall),
            SYSCALL_ID_MOUNT => Some(&MountSyscall),
            SYSCALL_ID_UMOUNT => Some(&UmountSyscall),
            SYSCALL_ID_MKDIRAT => Some(&MkdirAtSyscall),
            SYSCALL_ID_CHDIR => Some(&ChdirSyscall),
            SYSCALL_ID_NEWFSTATAT => Some(&NewFstatatSyscall),
            SYSCALL_ID_NEWFSTAT => Some(&NewFstatSyscall),
            SYSCALL_ID_GETDENTS64 => Some(&GetDents64Syscall),
            SYSCALL_ID_UNLINKAT => Some(&UnlinkAtSyscall),
            SYSCALL_ID_MMAP => Some(&MmapSyscall),
            SYSCALL_ID_MUNMAP => Some(&MunmapSyscall),
            SYSCALL_ID_CLOCK_GETTIME => Some(&ClockGetTimeSyscall),
            SYSCALL_ID_FCNTL64 => Some(&file::FileControlSyscall),
            SYSCALL_ID_IOCTL => Some(&file::IoControlSyscall),
            SYSCALL_ID_GETUID => Some(&GetRealUserIdSyscall),
            SYSCALL_ID_GETEUID => Some(&GetEffectiveUserIdSyscall),
            SYSCALL_ID_SYMLINKAT => Some(&SymbolLinkAtSyscall),
            SYSCALL_ID_LINKAT => Some(&LinkAtSyscall),
            SYSCALL_ID_READLINKAT => Some(&ReadLinkAtSyscall),
            SYSCALL_ID_SHUTDOWN => Some(&ShutdownSyscall),
            SYSCALL_ID_SYSLOG => Some(&SystemLogSyscall),
            SYSCALL_ID_SYSINFO => Some(&SystemInfoSyscall),
            SYSCALL_ID_GETRANDOM => Some(&GetRandomSyscall),
            SYSCALL_ID_PRLIMIT64 => Some(&ResourceLimitSyscall),
            SYSCALL_ID_GETTID => Some(&GetTaskIdSyscall),
            SYSCALL_ID_SET_TID_ADDRESS => Some(&GetTaskIdSyscall),
            SYSCALL_ID_LSEEK => Some(&LongSeekSyscall),
            SYSCALL_ID_FTRUNCATE64 => Some(&FileTruncateSyscall),
            SYSCALL_ID_SHMGET => Some(&SharedMemoryGetSyscall),
            SYSCALL_ID_SHMAT => Some(&SharedMemoryAttachSyscall),
            _ => None,
        }
    }

    pub async fn dispatch_async(
        tcb: &Arc<TaskControlBlock>,
        syscall_id: usize,
    ) -> Option<SyscallResult> {
        let mut ctx = SyscallContext::new(tcb);

        // Since interface with async function brokes object safety
        // The return value of a async function is actually a anonymous Type implementing Future
        // So we have to use static dispatch here
        match syscall_id {
            SYSCALL_ID_WRITE => Some(sys_write_async(&mut ctx).await),
            SYSCALL_ID_READ => Some(sys_read_async(&mut ctx).await),
            SYSCALL_ID_NANOSLEEP => Some(sys_nanosleep_async(&mut ctx).await),
            SYSCALL_ID_SCHED_YIELD => Some(sys_sched_yield_async(&mut ctx).await),
            STSCALL_ID_WAIT4 => Some(sys_wait4_async(&mut ctx).await),
            SYSCALL_ID_SENDFILE => Some(sys_sendfile_async(&mut ctx).await),
            SYSCALL_ID_WRITEV => Some(sys_writev_async(&mut ctx).await),
            SYSCALL_ID_READV => Some(sys_readv_async(&mut ctx).await),
            SYSCALL_ID_FUTEX => Some(sys_futex_async(&mut ctx).await),
            SYSCALL_ID_PSELECT6 => Some(sys_pselect6_async(&mut ctx).await),
            SYSCALL_ID_PPOLL => Some(sys_ppoll_async(&mut ctx).await),
            _ => None,
        }
    }
}

pub struct SyscallContext<'a> {
    tcb: &'a Arc<TaskControlBlock>,
    args: &'a [usize; 6],
}

impl<'a> SyscallContext<'a> {
    pub fn new(tcb: &'a Arc<TaskControlBlock>) -> Self {
        let args = unsafe { &*(&tcb.mut_trap_ctx().regs.a0 as *const usize as *const [usize; 6]) };
        SyscallContext { tcb, args }
    }
}

#[allow(unused)]
impl SyscallContext<'_> {
    #[inline]
    fn arg_i<T: Sized + Copy>(&self, i: usize) -> T {
        debug_assert!(core::mem::size_of::<T>() <= core::mem::size_of::<usize>());
        let arg = self.args[i];
        // Since RISCV is little-endian, we can safely cast usize to T
        unsafe { core::ptr::read(&arg as *const usize as *const T) }
    }

    #[inline]
    pub fn arg0<T: Sized + Copy>(&self) -> T {
        self.arg_i::<T>(0)
    }

    #[inline]
    pub fn arg1<T: Sized + Copy>(&self) -> T {
        self.arg_i::<T>(1)
    }

    #[inline]
    pub fn arg2<T: Sized + Copy>(&self) -> T {
        self.arg_i::<T>(2)
    }

    #[inline]
    pub fn arg3<T: Sized + Copy>(&self) -> T {
        self.arg_i::<T>(3)
    }

    #[inline]
    pub fn arg4<T: Sized + Copy>(&self) -> T {
        self.arg_i::<T>(4)
    }

    #[inline]
    pub fn arg5<T: Sized + Copy>(&self) -> T {
        self.arg_i::<T>(5)
    }
}

impl Deref for SyscallContext<'_> {
    type Target = Arc<TaskControlBlock>;

    fn deref(&self) -> &Self::Target {
        self.tcb
    }
}

pub trait ISyncSyscallHandler {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult;

    fn name(&self) -> &str;
}

// This is not recommended compare to async_syscall! macro
// It brokes debug line info, so better not use except for really simple syscall
#[macro_export]
macro_rules! sync_syscall {
    ($struct:ident, $syscall_name:expr, $param:ident, $body:block) => {
        pub struct $struct;

        impl $crate::syscalls::ISyncSyscallHandler for $struct {
            fn handle(&self, $param: &mut SyscallContext) -> SyscallResult {
                $body
            }

            fn name(&self) -> &str {
                $syscall_name
            }
        }
    };
}

#[macro_export]
macro_rules! async_syscall {
    ($name:ident, $param:ident, $body:block) => {
        pub async fn $name(
            $param: &mut $crate::syscalls::SyscallContext<'_>,
        ) -> $crate::syscalls::SyscallResult {
            // It's hard to find the syscall id constants with macro
            // So we just read the syscall id from the register
            let sys_id = $param.tcb.mut_trap_ctx().regs.a7;
            log::trace!(
                "[User trap] [Exception::Syscall] Async handler name: {}({}), task: {}({})",
                stringify!($name),
                sys_id,
                $param.tcb.task_id.id(),
                $param.tcb.pcb.lock().id
            );
            $body
        }
    };
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct UtsName {
    sysname: [u8; 65],
    nodename: [u8; 65],
    release: [u8; 65],
    version: [u8; 65],
    machine: [u8; 65],
    domainname: [u8; 65],
}

impl UtsName {
    fn write_to(&mut self, field: usize, text: &str) {
        let p_buf = unsafe { core::mem::transmute::<&mut UtsName, &mut [[u8; 65]; 6]>(self) };

        if field >= p_buf.len() {
            return;
        }

        let p_field = &mut p_buf[field];
        p_field.fill(0);

        let text = text.as_bytes();
        let len = core::cmp::min(
            text.len(),
            p_field.len() - 1, // reserved for null-terminated
        );

        p_field[..len].copy_from_slice(&text[..len]);
    }
}

struct UnameSyscall;

impl ISyncSyscallHandler for UnameSyscall {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult {
        let p_utsname = ctx.arg0::<*mut UtsName>();

        match ctx
            .borrow_page_table()
            .guard_ptr(p_utsname)
            .mustbe_user()
            .mustbe_readable()
            .with_write()
        {
            Some(mut guard) => {
                guard.write_to(0, "Linux");
                guard.write_to(1, "BakaOS");
                guard.write_to(2, "9.9.9");
                guard.write_to(3, &format!("#9 {}", constants::BUILD_TIME));
                guard.write_to(4, "RISC-IX");
                guard.write_to(5, "The most intelligent and strongest Cirno");

                Ok(0)
            }
            None => SyscallError::BadAddress,
        }
    }

    fn name(&self) -> &str {
        "sys_uname"
    }
}

sync_syscall!(GetRealUserIdSyscall, "sys_getuid", _ctx, { Ok(0) });

sync_syscall!(GetEffectiveUserIdSyscall, "sys_geteuid", _ctx, { Ok(0) });
