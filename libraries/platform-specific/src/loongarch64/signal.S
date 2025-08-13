.balign 4096
.section .text.trampoline_user
.globl __sigreturn_trampoline

__sigreturn_trampoline:
    li.d $a7, 139
    syscall 0
