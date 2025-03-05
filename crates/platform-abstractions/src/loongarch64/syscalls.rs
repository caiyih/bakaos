use core::ops::Deref;

use alloc::sync::Arc;
use tasks::TaskControlBlock;

use crate::{ISyscallContext, ISyscallContextBase};

pub struct LA64SyscallContext {
    tcb: Arc<TaskControlBlock>,
}

impl ISyscallContextBase for LA64SyscallContext {
    fn new(tcb: Arc<TaskControlBlock>) -> Self {
        Self { tcb }
    }

    fn move_to_next_instruction(&mut self) {
        self.mut_trap_ctx().era += 4
    }
}

impl LA64SyscallContext {
    #[inline]
    fn arg_i<T: Sized + Copy>(&self, i: usize) -> T {
        debug_assert!(core::mem::size_of::<T>() <= core::mem::size_of::<usize>());
        debug_assert!(i <= 5);

        let arg0 = unsafe {
            (self.tcb.mut_trap_ctx() as *const _ as *const usize).add(4 /* offset of a0 */)
        };

        // Since RISCV is little-endian, we can safely cast usize to T
        unsafe { arg0.add(i).cast::<T>().read() }
    }
}

impl ISyscallContext for LA64SyscallContext {
    #[inline(always)]
    fn syscall_id(&self) -> usize {
        self.tcb.mut_trap_ctx().regs.a7
    }

    #[inline(always)]
    fn arg0<T: Sized + Copy>(&self) -> T {
        self.arg_i(0)
    }

    #[inline(always)]
    fn arg1<T: Sized + Copy>(&self) -> T {
        self.arg_i(1)
    }

    #[inline(always)]
    fn arg2<T: Sized + Copy>(&self) -> T {
        self.arg_i(2)
    }

    #[inline(always)]
    fn arg3<T: Sized + Copy>(&self) -> T {
        self.arg_i(3)
    }

    #[inline(always)]
    fn arg4<T: Sized + Copy>(&self) -> T {
        self.arg_i(4)
    }

    #[inline(always)]
    fn arg5<T: Sized + Copy>(&self) -> T {
        self.arg_i(5)
    }
}

impl Deref for LA64SyscallContext {
    type Target = Arc<TaskControlBlock>;

    fn deref(&self) -> &Self::Target {
        &self.tcb
    }
}
