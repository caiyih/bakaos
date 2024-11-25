use core::{arch::asm, panic, usize};

use alloc::sync::Arc;
use log::{debug, trace};
use riscv::{
    interrupt::{
        supervisor::{Exception, Interrupt},
        Trap,
    },
    register::{sstatus, stvec},
};
use tasks::{TaskControlBlock, TaskStatus, TaskTrapContext};

use crate::syscalls::{ISyscallResult, SyscallDispatcher};

use super::set_kernel_trap_handler;

#[allow(unused)]
fn set_user_trap_handler() {
    trace!("Set trap handler to user");
    unsafe { stvec::write(__on_user_trap as usize, stvec::TrapMode::Direct) };
}

#[naked]
#[no_mangle]
#[link_section = ".text.trampoline_user"]
unsafe extern "C" fn __on_user_trap() {
    asm!(
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
        "ld s0,     36*8(sp)",
        "ld s1,     37*8(sp)",
        "ld s2,     38*8(sp)",
        "ld s3,     39*8(sp)",
        "ld s4,     40*8(sp)",
        "ld s5,     41*8(sp)",
        "ld s6,     42*8(sp)",
        "ld s7,     43*8(sp)",
        "ld s8,     44*8(sp)",
        "ld s9,     45*8(sp)",
        "ld s10,    46*8(sp)",
        "ld s11,    47*8(sp)",
        // Restore other kernel state
        "ld ra,     34*8(sp)",
        "ld tp,     35*8(sp)",
        "ld sp,     33*8(sp)", // Must restore sp at last
        "ret",                 // Return to kernel return address
        options(noreturn)
    );
}

#[naked]
#[no_mangle]
#[link_section = ".text.trampoline_user"]
unsafe extern "C" fn __return_from_user_trap(p_ctx: *mut TaskTrapContext) {
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
    asm!(
        "csrw sscratch, a0",
        // Saving kernel stack pointer
        "sd sp,     33*8(a0)",
        // Saving kernel return address
        "sd ra,     34*8(a0)",
        "sd tp,     35*8(a0)",
        // Save CoroutineSavedContext
        "sd s0,     36*8(a0)",
        "sd s1,     37*8(a0)",
        "sd s2,     38*8(a0)",
        "sd s3,     39*8(a0)",
        "sd s4,     40*8(a0)",
        "sd s5,     41*8(a0)",
        "sd s6,     42*8(a0)",
        "sd s7,     43*8(a0)",
        "sd s8,     44*8(a0)",
        "sd s9,     45*8(a0)",
        "sd s10,    46*8(a0)",
        "sd s11,    47*8(a0)",
        // Restore privilege registers
        "ld t0,     31*8(a0)",
        "ld t1,     32*8(a0)",
        "csrw sstatus, t0",
        "csrw sepc, t1",
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
        options(noreturn)
    );
}

pub fn return_to_user(tcb: &Arc<TaskControlBlock>) {
    trace!("Returning to task: {}", tcb.task_id.id());
    set_user_trap_handler();

    let ctx = tcb.trap_context.get();
    unsafe { sstatus::set_fs(sstatus::FS::Clean) };

    // TODO: Start stopwatch

    unsafe {
        __return_from_user_trap(ctx);
    }

    // TODO: Stop stopwatch and record the time

    set_kernel_trap_handler();
    unsafe { sstatus::set_sum() };

    trace!("Returned from task: {}", tcb.task_id.id());

    // return to task_loop, and then to user_trap_handler immediately
}

#[no_mangle]
pub async fn user_trap_handler_async(tcb: &Arc<TaskControlBlock>) {
    let scause = riscv::register::scause::read().cause();
    let stval = riscv::register::stval::read();

    trace!("[User trap] scause: {:?}, stval: {:#x}", scause, stval);

    let scause = unsafe { core::mem::transmute::<_, Trap<Interrupt, Exception>>(scause) };
    match scause {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            panic!("[User trap] [Interrupt::SupervisorTimer] Unimplemented")
        }
        Trap::Interrupt(i) => panic!("[User trap] [Interrupt] Unimplementd: {:?}", i),
        Trap::Exception(Exception::Breakpoint) => {
            #[cfg(debug_assertions)]
            debug!("[User trap] [Exception::Breakpoint]")
        }
        Trap::Exception(Exception::SupervisorEnvCall) => {
            panic!("[User trap] [Exception::SupervisorEnvCall]")
        }
        Trap::Exception(Exception::UserEnvCall) => {
            let trap_ctx = tcb.mut_trap_ctx();
            let syscall_id = trap_ctx.regs.a7;

            let ret = match SyscallDispatcher::dispatch(tcb, syscall_id) {
                Some((mut ctx, handler)) => {
                    debug!(
                        "[User trap] [Exception::Syscall] Sync handler name: {}({})",
                        handler.name(),
                        syscall_id,
                    );
                    handler.handle(&mut ctx).to_ret()
                }
                None => match SyscallDispatcher::dispatch_async(tcb, syscall_id).await {
                    Some(res) => res.to_ret(),
                    None => {
                        debug!(
                            "[User trap] [Exception::Syscall] Handler for id: {} not found. Kernel Killed it",
                            syscall_id
                        );
                        *tcb.task_status.lock() = TaskStatus::Exited;
                        return;
                    }
                },
            };
            trap_ctx.regs.a0 = ret as usize;
            trap_ctx.sepc += 4; // skip `ecall` instruction
        }
        Trap::Exception(e) => {
            // Trap::Exception(Exception::InstructionMisaligned) => (),
            // Trap::Exception(Exception::InstructionFault) => (),
            // Trap::Exception(Exception::IllegalInstruction) => (),
            // Trap::Exception(Exception::LoadMisaligned) => (),
            // Trap::Exception(Exception::LoadFault) => (),
            // Trap::Exception(Exception::StoreMisaligned) => (),
            // Trap::Exception(Exception::StoreFault) => (),
            // Trap::Exception(Exception::InstructionPageFault) => (),
            // Trap::Exception(Exception::LoadPageFault) => (),
            // Trap::Exception(Exception::StorePageFault) => (),
            debug!("[User Trap] [{:?}] Not supported! Kernel killed it", e);
            *tcb.task_status.lock() = TaskStatus::Exited;
        }
    }
}
