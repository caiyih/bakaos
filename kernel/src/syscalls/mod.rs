use alloc::sync::Arc;
use file::WriteSyscall;
use log::debug;
use tasks::{TaskControlBlock, TaskStatus};

mod file;

const SYSCALL_ID_WRITE: usize = 64;
const SYSCALL_ID_EXIT: usize = 93;

pub struct SyscallDispatcher;

impl SyscallDispatcher {
    fn translate_id(id: usize) -> Option<&'static dyn ISyscallHandler> {
        match id {
            SYSCALL_ID_WRITE => Some(&WriteSyscall),
            SYSCALL_ID_EXIT => Some(&ExitSyscall),
            _ => None,
        }
    }

    pub fn dispatch(
        tcb: &Arc<TaskControlBlock>,
        syscall_id: usize,
    ) -> Option<(SyscallContext, &'static dyn ISyscallHandler)> {
        let handler = Self::translate_id(syscall_id)?;

        let args = unsafe { &*(&tcb.mut_trap_ctx().regs.a0 as *const usize as *const [usize; 6]) };
        Some((SyscallContext { tcb, args }, handler))
    }
}

pub struct SyscallContext<'a> {
    pub tcb: &'a Arc<TaskControlBlock>,
    pub args: &'a [usize; 6],
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

pub trait ISyscallHandler {
    // TODO: Asynchronous syscalls
    fn handle(&self, ctx: &mut SyscallContext) -> isize;

    fn name(&self) -> &str;
}

struct ExitSyscall;

impl ISyscallHandler for ExitSyscall {
    fn handle(&self, ctx: &mut SyscallContext<'_>) -> isize {
        let code = ctx.arg0::<isize>();

        *ctx.tcb.task_status.lock() = TaskStatus::Exited;
        ctx.tcb
            .exit_code
            .store(code as i32, core::sync::atomic::Ordering::Relaxed);

        debug!("Task {} exited with code {}", ctx.tcb.task_id.id(), code);
        0
    }

    fn name(&self) -> &str {
        "sys_exit"
    }
}
