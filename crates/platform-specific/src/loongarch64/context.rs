use crate::ITaskContext;
use core::fmt::{self, Debug, Formatter};

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
pub struct FloatRegisterContext {
    pub f0: f64,
    pub f1: f64,
    pub f2: f64,
    pub f3: f64,
    pub f4: f64,
    pub f5: f64,
    pub f6: f64,
    pub f7: f64,
    pub f8: f64,
    pub f9: f64,
    pub f10: f64,
    pub f11: f64,
    pub f12: f64,
    pub f13: f64,
    pub f14: f64,
    pub f15: f64,
    pub f16: f64,
    pub f17: f64,
    pub f18: f64,
    pub f19: f64,
    pub f20: f64,
    pub f21: f64,
    pub f22: f64,
    pub f23: f64,
    pub f24: f64,
    pub f25: f64,
    pub f26: f64,
    pub f27: f64,
    pub f28: f64,
    pub f29: f64,
    pub f30: f64,
    pub f31: f64,
    pub fcc: u64,
    pub fcsr0: u32,
    pub dirty: bool,
    pub activated: bool,
}

impl FloatRegisterContext {
    pub fn activate_restore(&mut self) {
        self.restore();
        self.activated = true;
    }

    pub fn deactivate(&mut self) {
        self.snapshot();
        self.activated = false;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
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
    pub fregs: FloatRegisterContext,
}

impl Debug for TaskTrapContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskTrapContext")
            .field("regs", &self.regs)
            .field("prmd", &self.prmd)
            .field("era", &self.era)
            .field("badv", &self.badv)
            .field("crmd", &self.crmd)
            .finish()
    }
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

impl FloatRegisterContext {
    pub fn snapshot(&mut self) {
        if self.dirty {
            self.dirty = false;

            unsafe {
                core::arch::asm!(
                    "dbar 0",
                    "fst.d          $f0,    $a0,  0*8",
                    "fst.d          $f1,    $a0,  1*8",
                    "fst.d          $f2,    $a0,  2*8",
                    "fst.d          $f3,    $a0,  3*8",
                    "fst.d          $f4,    $a0,  4*8",
                    "fst.d          $f5,    $a0,  5*8",
                    "fst.d          $f6,    $a0,  6*8",
                    "fst.d          $f7,    $a0,  7*8",
                    "fst.d          $f8,    $a0,  8*8",
                    "fst.d          $f9,    $a0,  9*8",
                    "fst.d          $f10,   $a0, 10*8",
                    "fst.d          $f11,   $a0, 11*8",
                    "fst.d          $f12,   $a0, 12*8",
                    "fst.d          $f13,   $a0, 13*8",
                    "fst.d          $f14,   $a0, 14*8",
                    "fst.d          $f15,   $a0, 15*8",
                    "fst.d          $f16,   $a0, 16*8",
                    "fst.d          $f17,   $a0, 17*8",
                    "fst.d          $f18,   $a0, 18*8",
                    "fst.d          $f19,   $a0, 19*8",
                    "fst.d          $f20,   $a0, 20*8",
                    "fst.d          $f21,   $a0, 21*8",
                    "fst.d          $f22,   $a0, 22*8",
                    "fst.d          $f23,   $a0, 23*8",
                    "fst.d          $f24,   $a0, 24*8",
                    "fst.d          $f25,   $a0, 25*8",
                    "fst.d          $f26,   $a0, 26*8",
                    "fst.d          $f27,   $a0, 27*8",
                    "fst.d          $f28,   $a0, 28*8",
                    "fst.d          $f29,   $a0, 29*8",
                    "fst.d          $f30,   $a0, 30*8",
                    "fst.d          $f31,   $a0, 31*8",
                    "movcf2gr       $t0,    $fcc0",
                    "st.b           $t0,    $a0, 32*8+0",
                    "movcf2gr       $t0,    $fcc1",
                    "st.b           $t0,    $a0, 32*8+1",
                    "movcf2gr       $t0,    $fcc2",
                    "st.b           $t0,    $a0, 32*8+2",
                    "movcf2gr       $t0,    $fcc3",
                    "st.b           $t0,    $a0, 32*8+3",
                    "movcf2gr       $t0,    $fcc4",
                    "st.b           $t0,    $a0, 32*8+4",
                    "movcf2gr       $t0,    $fcc5",
                    "st.b           $t0,    $a0, 32*8+5",
                    "movcf2gr       $t0,    $fcc6",
                    "st.b           $t0,    $a0, 32*8+6",
                    "movcf2gr       $t0,    $fcc7",
                    "st.b           $t0,    $a0, 32*8+7",
                    "movfcsr2gr     $t0,    $fcsr0",
                    "st.d           $t0,    $a0, 33*8",
                    in("$a0") self,
                );
            }
        }
    }

    pub fn restore(&mut self) {
        if !self.activated {
            self.activated = true;

            unsafe {
                core::arch::asm!(
                    "ld.b           $t0,    $a0, 32*8+0",
                    "movgr2cf       $fcc0,  $t0",
                    "ld.b           $t0,    $a0, 32*8+1",
                    "movgr2cf       $fcc1,  $t0",
                    "ld.b           $t0,    $a0, 32*8+2",
                    "movgr2cf       $fcc2,  $t0",
                    "ld.b           $t0,    $a0, 32*8+3",
                    "movgr2cf       $fcc3,  $t0",
                    "ld.b           $t0,    $a0, 32*8+4",
                    "movgr2cf       $fcc4,  $t0",
                    "ld.b           $t0,    $a0, 32*8+5",
                    "movgr2cf       $fcc5,  $t0",
                    "ld.b           $t0,    $a0, 32*8+6",
                    "movgr2cf       $fcc6,  $t0",
                    "ld.b           $t0,    $a0, 32*8+7",
                    "movgr2cf       $fcc7,  $t0",
                    "fld.d          $f0,    $a0, 0*8",
                    "fld.d          $f1,    $a0, 1*8",
                    "fld.d          $f2,    $a0, 2*8",
                    "fld.d          $f3,    $a0, 3*8",
                    "fld.d          $f4,    $a0, 4*8",
                    "fld.d          $f5,    $a0, 5*8",
                    "fld.d          $f6,    $a0, 6*8",
                    "fld.d          $f7,    $a0, 7*8",
                    "fld.d          $f8,    $a0, 8*8",
                    "fld.d          $f9,    $a0, 9*8",
                    "fld.d          $f10,   $a0, 10*8",
                    "fld.d          $f11,   $a0, 11*8",
                    "fld.d          $f12,   $a0, 12*8",
                    "fld.d          $f13,   $a0, 13*8",
                    "fld.d          $f14,   $a0, 14*8",
                    "fld.d          $f15,   $a0, 15*8",
                    "fld.d          $f16,   $a0, 16*8",
                    "fld.d          $f17,   $a0, 17*8",
                    "fld.d          $f18,   $a0, 18*8",
                    "fld.d          $f19,   $a0, 19*8",
                    "fld.d          $f20,   $a0, 20*8",
                    "fld.d          $f21,   $a0, 21*8",
                    "fld.d          $f22,   $a0, 22*8",
                    "fld.d          $f23,   $a0, 23*8",
                    "fld.d          $f24,   $a0, 24*8",
                    "fld.d          $f25,   $a0, 25*8",
                    "fld.d          $f26,   $a0, 26*8",
                    "fld.d          $f27,   $a0, 27*8",
                    "fld.d          $f28,   $a0, 28*8",
                    "fld.d          $f29,   $a0, 29*8",
                    "fld.d          $f30,   $a0, 30*8",
                    "fld.d          $f31,   $a0, 31*8",
                    "ld.d           $t0,    $a0, 33*8",
                    "movgr2fcsr     $fcsr0, $t0",
                    in("$a0") self,
                );
            }
        }
    }
}
