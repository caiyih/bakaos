use core::ops::Deref;

use alloc::sync::Arc;
use tasks::TaskControlBlock;

pub trait ISyscallContextBase {
    fn new(tcb: Arc<TaskControlBlock>) -> Self;

    fn move_to_next_instruction(&mut self);
}

pub trait ISyscallContext: ISyscallContextBase + Deref<Target = Arc<TaskControlBlock>> {
    fn syscall_id(&self) -> usize;

    fn arg0<T: Sized + Copy>(&self) -> T;
    fn arg1<T: Sized + Copy>(&self) -> T;
    fn arg2<T: Sized + Copy>(&self) -> T;
    fn arg3<T: Sized + Copy>(&self) -> T;
    fn arg4<T: Sized + Copy>(&self) -> T;
    fn arg5<T: Sized + Copy>(&self) -> T;
}
