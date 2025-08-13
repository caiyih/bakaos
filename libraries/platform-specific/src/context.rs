pub trait ITaskContext {
    fn new(
        entry_pc: usize,
        stack_top: usize,
        argc: usize,
        argv_base: usize,
        envp_base: usize,
    ) -> Self;
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TestTaskContext {
    pub stack_top: usize,
    pub entry_pc: usize,
    pub return_value: usize,
}

impl TestTaskContext {
    #[allow(unused)]
    pub(crate) fn set_stack_top_internal(&mut self, stack_top: usize) {
        self.stack_top = stack_top;
    }

    #[allow(unused)]
    pub(crate) fn set_return_value_internal(&mut self, ret: usize) {
        self.return_value = ret;
    }
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
}
