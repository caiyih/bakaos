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
pub struct TestTaskContext {
    stack_top: usize,
    entry_pc: usize,
    return_value: usize,
}

impl ITaskContext for TestTaskContext {
    fn new(
        entry_pc: usize,
        stack_top: usize,
        _argc: usize,
        _argv_base: usize,
        _envp_base: usize,
    ) -> Self {
        Self {
            stack_top,
            entry_pc,
            return_value: 0,
        }
    }

    fn set_stack_top(&mut self, stack_top: usize) {
        self.stack_top = stack_top;
    }

    fn set_syscall_return_value(&mut self, ret: usize) {
        self.return_value = ret;
    }
}
