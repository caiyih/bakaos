use core::ops::{Deref, DerefMut};

use crate::TaskTrapContext;

pub struct SyscallContext<'a, TPayload> {
    pub(crate) trap_ctx: &'a mut TaskTrapContext,
    pub(crate) payload: TPayload,
}

impl<'a, TPayload> SyscallContext<'a, TPayload> {
    pub fn new(trap_ctx: &'a mut TaskTrapContext, payload: TPayload) -> Self {
        Self { trap_ctx, payload }
    }
}

pub trait ISyscallContext {
    fn syscall_id(&self) -> usize;

    fn arg0<T: Sized + Copy>(&self) -> T;
    fn arg1<T: Sized + Copy>(&self) -> T;
    fn arg2<T: Sized + Copy>(&self) -> T;
    fn arg3<T: Sized + Copy>(&self) -> T;
    fn arg4<T: Sized + Copy>(&self) -> T;
    fn arg5<T: Sized + Copy>(&self) -> T;
}

pub trait ISyscallContextMut {
    fn move_to_next_instruction(&mut self);
    fn set_return_value(&mut self, value: usize);
}

impl<TPayload> Deref for SyscallContext<'_, TPayload> {
    type Target = TPayload;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl<TPayload> DerefMut for SyscallContext<'_, TPayload> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.payload
    }
}
