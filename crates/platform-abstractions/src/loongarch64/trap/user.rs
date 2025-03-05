use core::arch::naked_asm;

use alloc::{boxed::Box, sync::Arc};
use loongArch64::register::{
    badv,
    estat::{self, Exception, Interrupt, Trap},
};
use platform_specific::TaskTrapContext;
use tasks::TaskControlBlock;

use crate::UserInterrupt;

#[naked]
#[no_mangle]
unsafe extern "C" fn __on_user_trap() {
    naked_asm!(
        "
            csrwr   $sp,  {KSAVE_USP}       // Save user sp into SAVE1 CSR
            csrrd   $sp,  {KSAVE_CTX}       // Restore trap context pointer

            st.d    $tp,  $sp, 2*8          // Save user tp and r21
            st.d    $r21, $sp, 21*8

            csrrd   $r21, {KSAVE_U0}        // Restore kernel u0
            csrrd   $tp,  {KSAVE_TP}        // Restore kernel tp

            csrrd   $t0, {KSAVE_T0}         // Restore and save t0
            st.d    $t0, $sp, 12*8          // to context

            csrrd   $t0, {KSAVE_USP}        // Save user tp
            st.d    $t0, $sp, 3*8           // to context

            st.d    $ra, $sp, 8
            st.d    $a0, $sp, 4*8
            st.d    $a1, $sp, 5*8
            st.d    $a2, $sp, 6*8
            st.d    $a3, $sp, 7*8
            st.d    $a4, $sp, 8*8
            st.d    $a5, $sp, 9*8
            st.d    $a6, $sp, 10*8
            st.d    $a7, $sp, 11*8
            st.d    $t1, $sp, 13*8
            st.d    $t2, $sp, 14*8
            st.d    $t3, $sp, 15*8
            st.d    $t4, $sp, 16*8
            st.d    $t5, $sp, 17*8
            st.d    $t6, $sp, 18*8
            st.d    $t7, $sp, 19*8
            st.d    $t8, $sp, 20*8

            st.d    $fp, $sp, 22*8
            st.d    $s0, $sp, 23*8
            st.d    $s1, $sp, 24*8
            st.d    $s2, $sp, 25*8
            st.d    $s3, $sp, 26*8
            st.d    $s4, $sp, 27*8
            st.d    $s5, $sp, 28*8
            st.d    $s6, $sp, 29*8
            st.d    $s7, $sp, 30*8
            st.d    $s8, $sp, 31*8

            csrrd	$t2, 0x1
            st.d	$t2, $sp, 8*32  // prmd
            csrrd   $t1, 0x6        
            st.d    $t1, $sp, 8*33  // era
            csrrd   $t1, 0x7   
            st.d    $t1, $sp, 8*34  // badv  
            csrrd   $t1, 0x0   
            st.d    $t1, $sp, 8*35  // crmd

            // Restore kernel coroutine saved context from *tp
            ld.d    $s0, $tp, 0*8
            ld.d    $s1, $tp, 1*8
            ld.d    $s2, $tp, 2*8
            ld.d    $s3, $tp, 3*8
            ld.d    $s4, $tp, 4*8
            ld.d    $s5, $tp, 5*8
            ld.d    $s6, $tp, 6*8
            ld.d    $s7, $tp, 7*8
            ld.d    $s8, $tp, 8*8
            ld.d    $fp, $tp, 9*8

            ld.d    $ra, $tp, 10*8
            ld.d    $sp, $tp, 11*8

            // return to the next instruction of calling __return_to_user
            // Then return to user task loop
            ret
        ",
        KSAVE_CTX = const 0x30,
        KSAVE_T0 = const  0x31,
        KSAVE_USP = const 0x32,
        KSAVE_U0 = const 0x33,
        KSAVE_TP = const  0x34,
    )
}

#[naked]
unsafe extern "C" fn __return_to_user(p_ctx: *mut TaskTrapContext) {
    naked_asm!(
        "
            // Save kernel coroutine context
            st.d    $s0, $tp, 0*8
            st.d    $s1, $tp, 1*8
            st.d    $s2, $tp, 2*8
            st.d    $s3, $tp, 3*8
            st.d    $s4, $tp, 4*8
            st.d    $s5, $tp, 5*8
            st.d    $s6, $tp, 6*8
            st.d    $s7, $tp, 7*8
            st.d    $s8, $tp, 8*8
            st.d    $fp, $tp, 9*8
            
            st.d    $ra, $tp, 10*8
            st.d    $sp, $tp, 11*8

            csrwr   $r21, {KSAVE_U0}        // Save kernel u0
            csrwr   $tp,  {KSAVE_TP}        // Save kernel tp
            
            csrwr   $a0,  {KSAVE_CTX}       // Save the pointer to user trap context

            // dbar 0

            ld.d    $t1, $a0, 8*33  // era
            csrwr   $t1, 0x6
            ld.d    $t2, $a0, 8*32  // prmd
            csrwr   $t2, 0x1

            ld.d    $ra, $a0, 1*8
            ld.d    $tp, $a0, 2*8
            ld.d    $sp, $a0, 3*8

            ld.d    $a1, $a0, 5*8
            ld.d    $a2, $a0, 6*8
            ld.d    $a3, $a0, 7*8
            ld.d    $a4, $a0, 8*8
            ld.d    $a5, $a0, 9*8
            ld.d    $a6, $a0, 10*8
            ld.d    $a7, $a0, 11*8
            ld.d    $t0, $a0, 12*8
            ld.d    $t1, $a0, 13*8
            ld.d    $t2, $a0, 14*8
            ld.d    $t3, $a0, 15*8
            ld.d    $t4, $a0, 16*8
            ld.d    $t5, $a0, 17*8
            ld.d    $t6, $a0, 18*8
            ld.d    $t7, $a0, 19*8
            ld.d    $t8, $a0, 20*8
            ld.d    $r21,$a0, 21*8
            ld.d    $fp, $a0, 22*8
            ld.d    $s0, $a0, 23*8
            ld.d    $s1, $a0, 24*8
            ld.d    $s2, $a0, 25*8
            ld.d    $s3, $a0, 26*8
            ld.d    $s4, $a0, 27*8
            ld.d    $s5, $a0, 28*8
            ld.d    $s6, $a0, 29*8
            ld.d    $s7, $a0, 30*8
            ld.d    $s8, $a0, 31*8

            // restore a0
            ld.d    $a0, $a0, 4*8

            // return to user space
            ertn
        ",
        KSAVE_CTX = const 0x30,
        KSAVE_U0 = const 0x33,
        KSAVE_TP = const  0x34,
    )
}

#[no_mangle]
pub extern "C" fn return_to_user(tcb: &Arc<TaskControlBlock>) {
    let ctx = tcb.trap_context.get();

    unsafe {
        __return_to_user(ctx);
    }
}

pub fn translate_current_trap() -> UserInterrupt {
    let estat = estat::read();
    let badv = badv::read();

    let cause = estat.cause();

    match cause {
        Trap::Interrupt(Interrupt::Timer) => UserInterrupt::Timer,
        Trap::Interrupt(_) => {
            let irq_num: usize = estat.is().trailing_zeros() as usize;
            UserInterrupt::Irq(irq_num)
        }
        Trap::Exception(Exception::Syscall) => UserInterrupt::Syscall,
        Trap::Exception(Exception::Breakpoint) => UserInterrupt::Breakpoint,
        Trap::MachineError(error) => UserInterrupt::Unknown(Box::new(error)),
        Trap::Unknown => UserInterrupt::Unknown(Box::new(())),
        Trap::Exception(
            Exception::LoadPageFault | Exception::FetchPageFault | Exception::PageNonReadableFault,
        ) => UserInterrupt::LoadPageFault(badv.vaddr()),
        Trap::Exception(Exception::StorePageFault | Exception::PageModifyFault) => {
            UserInterrupt::StorePageFault(badv.vaddr())
        }
        Trap::Exception(Exception::PageNonExecutableFault) => {
            UserInterrupt::InstructionPageFault(badv.vaddr())
        }
        Trap::Exception(Exception::FetchInstructionAddressError) => {
            UserInterrupt::InstructionMisaligned(badv.vaddr())
        }
        Trap::Exception(
            Exception::InstructionNotExist
            | Exception::InstructionPrivilegeIllegal
            | Exception::FloatingPointUnavailable,
        ) => UserInterrupt::IllegalInstruction(badv.vaddr()),
        Trap::Exception(
            Exception::AddressNotAligned
            | Exception::BoundsCheckFault
            | Exception::PagePrivilegeIllegal
            | Exception::MemoryAccessAddressError,
        ) => UserInterrupt::AccessFault(badv.vaddr()),
        _ => UserInterrupt::Unknown(Box::new(cause)),
    }
}
