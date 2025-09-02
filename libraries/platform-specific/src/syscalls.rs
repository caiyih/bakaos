use core::ops::{Deref, DerefMut};

use trap_abstractions::ITaskTrapContext;

pub struct SyscallPayload<'a, TPayload> {
    pub trap_ctx: &'a mut dyn ITaskTrapContext,
    pub payload: TPayload,
}

impl<'a, TPayload> SyscallPayload<'a, TPayload> {
    pub fn new(trap_ctx: &'a mut dyn ITaskTrapContext, payload: TPayload) -> Self {
        SyscallPayload { trap_ctx, payload }
    }
}

impl<TPayload> Deref for SyscallPayload<'_, TPayload> {
    type Target = TPayload;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl<TPayload> DerefMut for SyscallPayload<'_, TPayload> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.payload
    }
}
