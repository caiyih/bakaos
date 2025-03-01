use crate::ITaskContext;

#[derive(Debug, Clone, Copy)]
pub struct TaskTrapContext {
    /// general registers.
    pub regs: [usize; 32],
    /// Pre-exception Mode Information
    pub prmd: usize,
    /// Exception Return Address
    pub era: usize,
    /// Access Memory Address When Exception
    pub badv: usize,
    /// Current Mode Information
    pub crmd: usize,
    // TODO: Add kernel coroutine context and float point context
}

impl ITaskContext for TaskTrapContext {
    fn new(
        entry_pc: usize,
        stack_top: usize,
        _argc: usize,
        argv_base: usize,
        _envp_base: usize,
    ) -> Self {
        const PPLV_UMODE: usize = 0b11;
        const PIE: usize = 1 << 2;
        let mut ctx = unsafe { core::mem::zeroed::<Self>() };

        ctx.regs[3] = stack_top;
        ctx.era = entry_pc;
        ctx.prmd = PPLV_UMODE | PIE;

        ctx.regs[4] = argv_base;

        ctx
    }

    fn set_stack_top(&mut self, stack_top: usize) {
        self.regs[3] = stack_top;
    }

    fn set_syscall_return_value(&mut self, ret: usize) {
        self.regs[4] = ret;
    }
}
