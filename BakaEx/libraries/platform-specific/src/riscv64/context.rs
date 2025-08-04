use riscv::register::sstatus::{self, Sstatus, FS, SPP};

use crate::ITaskContext;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GeneralRegisterContext {
    // Not saving x0 for simplicity
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub fp: usize, // s0
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FloatRegisterContext {
    pub f: [f64; 32], // 0 - 31
    pub fcsr: u32,
    pub dirty: bool,
    pub activated: bool,
}

impl FloatRegisterContext {
    pub fn snapshot(&mut self) {
        if self.dirty {
            self.dirty = false;

            unsafe {
                core::arch::asm!(
                    "fsd    f0,     0*8(a0)",
                    "fsd    f1,     1*8(a0)",
                    "fsd    f2,     2*8(a0)",
                    "fsd    f3,     3*8(a0)",
                    "fsd    f4,     4*8(a0)",
                    "fsd    f5,     5*8(a0)",
                    "fsd    f6,     6*8(a0)",
                    "fsd    f7,     7*8(a0)",
                    "fsd    f8,     8*8(a0)",
                    "fsd    f9,     9*8(a0)",
                    "fsd    f10,   10*8(a0)",
                    "fsd    f11,   11*8(a0)",
                    "fsd    f12,   12*8(a0)",
                    "fsd    f13,   13*8(a0)",
                    "fsd    f14,   14*8(a0)",
                    "fsd    f15,   15*8(a0)",
                    "fsd    f16,   16*8(a0)",
                    "fsd    f17,   17*8(a0)",
                    "fsd    f18,   18*8(a0)",
                    "fsd    f19,   19*8(a0)",
                    "fsd    f20,   20*8(a0)",
                    "fsd    f21,   21*8(a0)",
                    "fsd    f22,   22*8(a0)",
                    "fsd    f23,   23*8(a0)",
                    "fsd    f24,   24*8(a0)",
                    "fsd    f25,   25*8(a0)",
                    "fsd    f26,   26*8(a0)",
                    "fsd    f27,   27*8(a0)",
                    "fsd    f28,   28*8(a0)",
                    "fsd    f29,   29*8(a0)",
                    "fsd    f30,   30*8(a0)",
                    "fsd    f31,   31*8(a0)",
                    "csrr   t0,    fcsr",
                    "sw     t0,    31*8(a0)",
                    in("a0") self,
                    options(nostack)
                );
            }
        }
    }

    pub fn restore(&mut self) {
        if !self.activated {
            self.activated = true;

            unsafe {
                core::arch::asm!(
                    "fld    f0,     0*8(a0)",
                    "fld    f1,     1*8(a0)",
                    "fld    f2,     2*8(a0)",
                    "fld    f3,     3*8(a0)",
                    "fld    f4,     4*8(a0)",
                    "fld    f5,     5*8(a0)",
                    "fld    f6,     6*8(a0)",
                    "fld    f7,     7*8(a0)",
                    "fld    f8,     8*8(a0)",
                    "fld    f9,     9*8(a0)",
                    "fld    f10,   10*8(a0)",
                    "fld    f11,   11*8(a0)",
                    "fld    f12,   12*8(a0)",
                    "fld    f13,   13*8(a0)",
                    "fld    f14,   14*8(a0)",
                    "fld    f15,   15*8(a0)",
                    "fld    f16,   16*8(a0)",
                    "fld    f17,   17*8(a0)",
                    "fld    f18,   18*8(a0)",
                    "fld    f19,   19*8(a0)",
                    "fld    f20,   20*8(a0)",
                    "fld    f21,   21*8(a0)",
                    "fld    f22,   22*8(a0)",
                    "fld    f23,   23*8(a0)",
                    "fld    f24,   24*8(a0)",
                    "fld    f25,   25*8(a0)",
                    "fld    f26,   26*8(a0)",
                    "fld    f27,   27*8(a0)",
                    "fld    f28,   28*8(a0)",
                    "fld    f29,   29*8(a0)",
                    "fld    f30,   30*8(a0)",
                    "fld    f31,   31*8(a0)",
                    "lw     t0,    32*8(a0)",
                    "csrw   fcsr,  t0",
                    in("a0") self,
                    options(nostack)
                );
            }
        }
    }

    pub fn on_trap(&mut self, sstatus: Sstatus) {
        self.dirty |= sstatus.fs() == FS::Dirty;
    }

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
#[derive(Clone, Copy, Debug)]
pub struct TaskTrapContext {
    pub regs: GeneralRegisterContext, // 0 - 30
    pub sstatus: usize,               // 31
    pub sepc: usize,                  // 32
    ktp: usize,                       // kernel thread info pointer, 33
    pub fregs: FloatRegisterContext,  // Dont' rename, cross crates inter-operation
}

impl TaskTrapContext {
    pub(crate) fn set_stack_top_internal(&mut self, stack_top: usize) {
        self.regs.sp = stack_top
    }

    pub(crate) fn set_return_value_internal(&mut self, ret: usize) {
        self.regs.a0 = ret
    }
}

impl ITaskContext for TaskTrapContext {
    fn new(
        entry_pc: usize,
        stack_top: usize,
        _argc: usize,
        _argv_base: usize,
        _envp_base: usize,
    ) -> Self {
        let mut ctx = unsafe { core::mem::zeroed::<TaskTrapContext>() };
        ctx.sepc = entry_pc;
        ctx.regs.sp = stack_top;

        let sstatus = sstatus::read();
        unsafe {
            sstatus::set_spp(SPP::User);
            sstatus::set_sum();
        }

        ctx.sstatus = unsafe { core::mem::transmute::<Sstatus, usize>(sstatus) };

        ctx.regs.a0 = 0;

        ctx
    }
}
