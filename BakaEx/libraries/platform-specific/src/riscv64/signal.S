.balign 4096
.section .text.trampoline_user
.globl __sigreturn_trampoline

__sigreturn_trampoline:
    li a7, 139
    ecall
