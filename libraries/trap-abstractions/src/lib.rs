#![cfg_attr(not(feature = "std"), no_std)]

use downcast_rs::{impl_downcast, Downcast};

#[cfg(feature = "std")]
extern crate std;

pub trait ITaskTrapContext: Downcast {
    fn copy_from(&mut self, other: &dyn ITaskTrapContext);

    fn set_stack_top(&mut self, stack_top: usize);

    fn set_return_value(&mut self, ret: usize);
}

impl_downcast!(ITaskTrapContext);

pub trait ISyscallPayload {
    fn syscall_id(&self) -> usize;

    fn arg0<T: Sized + Copy>(&self) -> T;
    fn arg1<T: Sized + Copy>(&self) -> T;
    fn arg2<T: Sized + Copy>(&self) -> T;
    fn arg3<T: Sized + Copy>(&self) -> T;
    fn arg4<T: Sized + Copy>(&self) -> T;
    fn arg5<T: Sized + Copy>(&self) -> T;
}

pub trait ISyscallPayloadMut {
    fn move_to_next_instruction(&mut self);
}
