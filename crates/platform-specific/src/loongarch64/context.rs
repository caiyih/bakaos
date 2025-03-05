use crate::ITaskContext;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GeneralRegisterContext {
    pub r0: usize, //  0
    pub ra: usize, //  1
    pub tp: usize, //  2
    pub sp: usize, //  3
    pub a0: usize, //  4
    pub a1: usize, //  5
    pub a2: usize, //  6
    pub a3: usize, //  7
    pub a4: usize, //  8
    pub a5: usize, //  9
    pub a6: usize, // 10
    pub a7: usize, // 11
    pub t0: usize, // 12
    pub t1: usize, // 13
    pub t2: usize, // 14
    pub t3: usize, // 15
    pub t4: usize, // 16
    pub t5: usize, // 17
    pub t6: usize, // 18
    pub t7: usize, // 19
    pub t8: usize, // 20
    pub u0: usize, // 21
    pub fp: usize, // 22
    pub s0: usize, // 23
    pub s1: usize, // 24
    pub s2: usize, // 25
    pub s3: usize, // 26
    pub s4: usize, // 27
    pub s5: usize, // 28
    pub s6: usize, // 29
    pub s7: usize, // 30
    pub s8: usize, // 31
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskTrapContext {
    /// general registers.
    pub regs: GeneralRegisterContext, // 0 - 31
    /// Pre-exception Mode Information
    pub prmd: usize, // 32
    /// Exception Return Address
    pub era: usize, // 33
    /// Access Memory Address When Exception
    pub badv: usize, // 34
    /// Current Mode Information
    pub crmd: usize, // 35
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

        ctx.regs.sp = stack_top;
        ctx.era = entry_pc;
        ctx.prmd = PPLV_UMODE | PIE;

        ctx.regs.a0 = argv_base;

        ctx
    }

    fn set_stack_top(&mut self, stack_top: usize) {
        self.regs.sp = stack_top;
    }

    fn set_syscall_return_value(&mut self, ret: usize) {
        self.regs.a0 = ret;
    }
}
