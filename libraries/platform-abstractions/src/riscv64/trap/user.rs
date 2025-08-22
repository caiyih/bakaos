use core::{arch::naked_asm, panic};

use alloc::boxed::Box;
use platform_specific::TaskTrapContext;
use riscv::{
    interrupt::{
        supervisor::{Exception, Interrupt},
        Trap,
    },
    register::{
        sstatus::{self, Sstatus},
        stvec::{self},
    },
};
use trap_abstractions::ITaskTrapContext;

use crate::interrupts::UserInterrupt;

use super::set_kernel_trap_handler;

fn set_user_trap_handler() {
    unsafe { stvec::write(__on_user_trap as usize, stvec::TrapMode::Direct) };
}

#[naked]
#[no_mangle]
#[link_section = ".text.trampoline_user"]
unsafe extern "C" fn __on_user_trap() {
    naked_asm!(
        // Exchange sp with sscratch
        // So that sp points to TaskTrapContext, which is saved at the first line of __return_from_user_trap
        // And ssctatch points to the user stack
        "csrrw sp, sscratch, sp",
        // Snapshot general registers of user task
        "sd ra,      0*8(sp)",
        // "sd sp,   1*8(sp)", // Can not save sp, as it's used as base address of TaskTrapContext
        "sd gp,      2*8(sp)",
        "sd tp,      3*8(sp)",
        "sd t0,      4*8(sp)",
        "sd t1,      5*8(sp)",
        "sd t2,      6*8(sp)",
        "sd s0,      7*8(sp)", // aka. fp
        "sd s1,      8*8(sp)",
        "sd a0,      9*8(sp)",
        "sd a1,     10*8(sp)",
        "sd a2,     11*8(sp)",
        "sd a3,     12*8(sp)",
        "sd a4,     13*8(sp)",
        "sd a5,     14*8(sp)",
        "sd a6,     15*8(sp)",
        "sd a7,     16*8(sp)",
        "sd s2,     17*8(sp)",
        "sd s3,     18*8(sp)",
        "sd s4,     19*8(sp)",
        "sd s5,     20*8(sp)",
        "sd s6,     21*8(sp)",
        "sd s7,     22*8(sp)",
        "sd s8,     23*8(sp)",
        "sd s9,     24*8(sp)",
        "sd s10,    25*8(sp)",
        "sd s11,    26*8(sp)",
        "sd t3,     27*8(sp)",
        "sd t4,     28*8(sp)",
        "sd t5,     29*8(sp)",
        "sd t6,     30*8(sp)",
        // Now we can use registers except sp
        // But let's save privilege registers and sp first
        "csrr t0, sstatus",
        "sd   t0, 31*8(sp)",
        "csrr t0, sepc",
        "sd   t0, 32*8(sp)",
        "csrr t0, sscratch",
        "sd   t0, 1*8(sp)",
        // Restore CoroutineSavedContext
        // Basically a reverse order of saving
        // lets restore tp first, as other coroutine context are saved in *tp
        "ld tp,     33*8(sp)",
        "ld s0,      0*8(tp)",
        "ld s1,      1*8(tp)",
        "ld s2,      2*8(tp)",
        "ld s3,      3*8(tp)",
        "ld s4,      4*8(tp)",
        "ld s5,      5*8(tp)",
        "ld s6,      6*8(tp)",
        "ld s7,      7*8(tp)",
        "ld s8,      8*8(tp)",
        "ld s9,      9*8(tp)",
        "ld s10,    10*8(tp)",
        "ld s11,    11*8(tp)",
        // Restore other kernel state
        "ld ra,     12*8(tp)",
        "ld sp,     13*8(tp)", // Must restore sp at last
        "ret",                 // Return to kernel return address
    );
}

#[naked]
#[no_mangle]
pub unsafe extern "C" fn __return_to_user(p_ctx: &mut TaskTrapContext) {
    // Layout of TaskTrapContext, see src/tasks/user_task.rs for details:
    // +---------+
    // |   x1    |  <- a0
    // |   x2    |  <- a1 + 8
    // |   ...   |
    // +---------+
    // | sstatus |  <- a0 + 31 * 8
    // |  ksepc  |
    // |   ksp   |
    // |   kra   |
    // |   ktp   |
    // +---------+
    // |   ks0   |  <- a0 + 35 * 8
    // |   ks1   |
    // |   ...   |
    // +---------+
    naked_asm!(
        // Saving kernel return address
        "sd ra,     12*8(tp)",
        // Saving kernel stack pointer
        "sd sp,     13*8(tp)",
        // Save CoroutineSavedContext
        "sd s0,      0*8(tp)",
        "sd s1,      1*8(tp)",
        "sd s2,      2*8(tp)",
        "sd s3,      3*8(tp)",
        "sd s4,      4*8(tp)",
        "sd s5,      5*8(tp)",
        "sd s6,      6*8(tp)",
        "sd s7,      7*8(tp)",
        "sd s8,      8*8(tp)",
        "sd s9,      9*8(tp)",
        "sd s10,    10*8(tp)",
        "sd s11,    11*8(tp)",
        // Store thread info pointer in TaskTrapContext
        "sd tp,     33*8(a0)",
        // Restore privilege registers
        "ld t0,     31*8(a0)",
        "ld t1,     32*8(a0)",
        "csrw sstatus, t0",
        "csrw sepc, t1",
        // Keep a backup for pTaskTrapContext
        "csrw sscratch, a0",
        // Restore general registers of user task
        // x0        : zero      (Hard-wired zero, so we don't need to snapshot/restore it)
        // x1        : ra        (Returrn Address)
        // x2        : sp        (Stack Pointer)
        // x3        : gp        (Global Pointer)
        // x4        : tp        (Thread Pointer)
        // x5 - x7   : t0 - t2   (Temporary Registers 0 - 2)
        // x8        : s0/fp     (Saved Register 0/Frame Pointer)
        // x9        : s1        (Saved Register 1)
        // x10 - x11 : a0 - a1   (Return Values / Function Arguments 0 - 1)
        // x12 - x17 : a2 - a7   (Function Arguments 2 - 7)
        // x18 - x27 : s2 - s11  (Saved Registers 2 - 11)
        // x28 - x31 : t3 - t6   (Temporary Registers 3 - 6)
        // We restore a0 at last, as it's used as base address of TaskTrapContext
        "ld ra,      0*8(a0)",
        "ld sp,      1*8(a0)",
        "ld gp,      2*8(a0)",
        "ld tp,      3*8(a0)",
        "ld t0,      4*8(a0)",
        "ld t1,      5*8(a0)",
        "ld t2,      6*8(a0)",
        "ld s0,      7*8(a0)", // aka. fp
        "ld s1,      8*8(a0)",
        // "ld a0,   9*8(a0)", // Can not restore a0, as it is used as base address of TaskTrapContext
        "ld a1,     10*8(a0)",
        "ld a2,     11*8(a0)",
        "ld a3,     12*8(a0)",
        "ld a4,     13*8(a0)",
        "ld a5,     14*8(a0)",
        "ld a6,     15*8(a0)",
        "ld a7,     16*8(a0)",
        "ld s2,     17*8(a0)",
        "ld s3,     18*8(a0)",
        "ld s4,     19*8(a0)",
        "ld s5,     20*8(a0)",
        "ld s6,     21*8(a0)",
        "ld s7,     22*8(a0)",
        "ld s8,     23*8(a0)",
        "ld s9,     24*8(a0)",
        "ld s10,    25*8(a0)",
        "ld s11,    26*8(a0)",
        "ld t3,     27*8(a0)",
        "ld t4,     28*8(a0)",
        "ld t5,     29*8(a0)",
        "ld t6,     30*8(a0)",
        "ld a0,      9*8(a0)", // Now we can restore a0, as it's not used as base address of TaskTrapContext any more
        "sret",
    );
}

pub fn return_to_user(ctx: &mut dyn ITaskTrapContext) -> UserInterrupt {
    set_user_trap_handler();

    let ctx = unsafe { (ctx as *mut _ as *mut TaskTrapContext).as_mut().unwrap() };

    ctx.fregs.activate_restore(); // TODO: Should let the scheduler activate it
    unsafe { sstatus::set_fs(sstatus::FS::Clean) };

    // tcb.kernel_timer.lock().set();

    unsafe {
        __return_to_user(ctx);
    }

    // tcb.kernel_timer.lock().start();

    set_kernel_trap_handler();
    unsafe { sstatus::set_sum() };
    let sstatus = unsafe { core::mem::transmute::<usize, Sstatus>(ctx.sstatus) };
    ctx.fregs.on_trap(sstatus);
    ctx.fregs.deactivate(); // TODO: Should let the scheduler deactivate it

    // return to task_loop, and then to user_trap_handler immediately
    translate_current_trap()
}

pub fn translate_current_trap() -> UserInterrupt {
    let scause = riscv::register::scause::read().cause();
    let stval = riscv::register::stval::read();

    let scause =
        unsafe { core::mem::transmute::<Trap<usize, usize>, Trap<Interrupt, Exception>>(scause) };

    match scause {
        Trap::Exception(Exception::Breakpoint) => UserInterrupt::Breakpoint,
        Trap::Exception(Exception::UserEnvCall) => UserInterrupt::Syscall,

        Trap::Exception(Exception::IllegalInstruction) => UserInterrupt::IllegalInstruction(stval),

        Trap::Exception(Exception::InstructionMisaligned) => {
            UserInterrupt::InstructionMisaligned(stval)
        }
        Trap::Exception(Exception::InstructionPageFault) => {
            UserInterrupt::InstructionPageFault(stval)
        }
        Trap::Exception(Exception::InstructionFault) => UserInterrupt::AccessFault(stval),

        Trap::Exception(Exception::LoadFault) => UserInterrupt::AccessFault(stval),
        Trap::Exception(Exception::LoadPageFault) => UserInterrupt::LoadPageFault(stval),
        Trap::Exception(Exception::LoadMisaligned) => UserInterrupt::LoadPageFault(stval),

        Trap::Exception(Exception::StoreFault) => UserInterrupt::AccessFault(stval),
        Trap::Exception(Exception::StoreMisaligned) => UserInterrupt::StoreMisaligned(stval),
        Trap::Exception(Exception::StorePageFault) => UserInterrupt::StorePageFault(stval),

        Trap::Exception(Exception::SupervisorEnvCall) => {
            panic!("[User trap] [Exception::SupervisorEnvCall] This should never happen")
        }

        Trap::Interrupt(Interrupt::SupervisorTimer) => UserInterrupt::Timer,
        Trap::Interrupt(Interrupt::SupervisorExternal) => UserInterrupt::SupervisorExternal,

        _ => UserInterrupt::Unknown(Box::new(scause)),
    }
}
