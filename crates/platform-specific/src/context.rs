pub trait ITaskContext {
    fn new(
        entry_pc: usize,
        stack_top: usize,
        argc: usize,
        argv_base: usize,
        envp_base: usize,
    ) -> Self;

    fn set_stack_top(&mut self, stack_top: usize);

    fn set_syscall_return_value(&mut self, ret: usize);
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DummyTaskContext;

impl ITaskContext for DummyTaskContext {
    fn new(
        _entry_pc: usize,
        _stack_top: usize,
        _argc: usize,
        _argv_base: usize,
        _envp_base: usize,
    ) -> Self {
        Self
    }

    fn set_stack_top(&mut self, _stack_top: usize) {}

    fn set_syscall_return_value(&mut self, _ret: usize) {}
}
