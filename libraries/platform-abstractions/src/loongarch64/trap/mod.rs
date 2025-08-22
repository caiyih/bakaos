use core::arch::global_asm;

mod kernel;
mod user;

pub use user::{return_to_user, translate_current_trap};

global_asm!(
    "
        .section .text
        .balign 4096
        .global trap_vector_base
        trap_vector_base:
            csrwr   $t0, {KSAVE_T0}
            csrrd   $t0, 0x1
            andi    $t0, $t0, 0x3
            bnez    $t0, __on_user_trap
            b       __on_kernel_trap
    ",
    KSAVE_T0 = const 0x31
);
