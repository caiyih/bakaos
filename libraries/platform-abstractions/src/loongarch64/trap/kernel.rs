use core::arch::naked_asm;

use loongArch64::register::estat;
use platform_specific::TaskTrapContext;

#[naked]
#[no_mangle]
unsafe extern "C" fn __on_kernel_trap() {
    naked_asm!(
        "
            move    $t0, $sp  
            addi.d  $sp, $sp, -{trapframe_size} // allocate space
            // save kernel sp
            st.d    $t0, $sp, 3*8
            
            st.d    $ra, $sp, 8
            csrrd   $t0, {KSAVE_T0}
            st.d    $t0, $sp, 12*8

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

            move    $a0, $sp
            bl      kernel_trap_handler

            // TODO: this is unnecessary
            // restore the registers.
            ld.d    $t1, $sp, 8*33  // era
            csrwr   $t1, 0x6
            ld.d    $t2, $sp, 8*32  // prmd
            csrwr   $t2, 0x1

            ld.d    $ra, $sp, 1*8
            ld.d    $a0, $sp, 4*8
            ld.d    $a1, $sp, 5*8
            ld.d    $a2, $sp, 6*8
            ld.d    $a3, $sp, 7*8
            ld.d    $a4, $sp, 8*8
            ld.d    $a5, $sp, 9*8
            ld.d    $a6, $sp, 10*8
            ld.d    $a7, $sp, 11*8
            ld.d    $t0, $sp, 12*8
            ld.d    $t1, $sp, 13*8
            ld.d    $t2, $sp, 14*8
            ld.d    $t3, $sp, 15*8
            ld.d    $t4, $sp, 16*8
            ld.d    $t5, $sp, 17*8
            ld.d    $t6, $sp, 18*8
            ld.d    $t7, $sp, 19*8
            ld.d    $t8, $sp, 20*8

            ld.d    $fp, $sp, 22*8
            ld.d    $s0, $sp, 23*8
            ld.d    $s1, $sp, 24*8
            ld.d    $s2, $sp, 25*8
            ld.d    $s3, $sp, 26*8
            ld.d    $s4, $sp, 27*8
            ld.d    $s5, $sp, 28*8
            ld.d    $s6, $sp, 29*8
            ld.d    $s7, $sp, 30*8
            ld.d    $s8, $sp, 31*8

            // restore sp
            ld.d    $sp, $sp, 3*8
            ertn
        ",
        trapframe_size = const ::core::mem::size_of::<TaskTrapContext>(),
        KSAVE_T0 = const  0x31,
    )
}

#[no_mangle]
extern "C" fn kernel_trap_handler(ctx: &mut TaskTrapContext) {
    let estat = estat::read();

    panic!(
        "Unhandled kernel exception: {:?} @ {:#x}:\n{:#x?}",
        estat.cause(),
        ctx.era,
        ctx
    );
}
