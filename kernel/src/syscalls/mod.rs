use alloc::sync::Arc;
use file::WriteSyscall;
use paging::{IWithPageGuardBuilder, PageTableEntryFlags};
use task::{ExitSyscall, TimesSyscall};
use tasks::TaskControlBlock;

mod file;
mod task;

const SYSCALL_ID_WRITE: usize = 64;
const SYSCALL_ID_EXIT: usize = 93;
const SYSCALL_ID_TIMES: usize = 153;
const SYSCALL_ID_UNAME: usize = 160;
const SYSCALL_ID_BRK: usize = 214;

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
    fn translate_id(id: usize) -> Option<&'static dyn ISyncSyscallHandler> {
        match id {
            SYSCALL_ID_WRITE => Some(&WriteSyscall),
            SYSCALL_ID_EXIT => Some(&ExitSyscall),
            SYSCALL_ID_TIMES => Some(&TimesSyscall),
            SYSCALL_ID_UNAME => Some(&UnameSyscall),
            SYSCALL_ID_BRK => Some(&task::BrkSyscall),
            _ => None,
        }
    }

    pub fn dispatch(
        tcb: &Arc<TaskControlBlock>,
        syscall_id: usize,
    ) -> Option<(SyscallContext, &'static dyn ISyncSyscallHandler)> {
        let handler = Self::translate_id(syscall_id)?;

        Some((SyscallContext::new(tcb), handler))
    }

    pub async fn dispatch_async(
        _tcb: &Arc<TaskControlBlock>,
        _syscall_id: usize,
    ) -> Option<SyscallResult> {
        None
    }
}

pub struct SyscallContext<'a> {
    pub tcb: &'a Arc<TaskControlBlock>,
    pub args: &'a [usize; 6],
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

pub trait ISyncSyscallHandler {
    fn handle(&self, ctx: &mut SyscallContext) -> SyscallResult;

    fn name(&self) -> &str;
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
        let p_buf = unsafe { core::mem::transmute::<_, &mut [[u8; 65]; 6]>(self) };

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

        let memory_space = ctx.tcb.memory_space.lock();
        match memory_space
            .page_table()
            .guard_ptr(p_utsname)
            .must_have(PageTableEntryFlags::User | PageTableEntryFlags::Readable)
            .with(PageTableEntryFlags::Writable)
        {
            Some(mut guard) => {
                guard.write_to(0, "Linux");
                guard.write_to(1, "BakaOS");
                guard.write_to(2, "9.9.9");
                guard.write_to(3, "#9");
                guard.write_to(4, "RISC-IX");
                guard.write_to(5, "The most intelligent and strongest Cirno");

                Ok(0)
            }
            None => Err(-1),
        }
    }

    fn name(&self) -> &str {
        "sys_uname"
    }
}
