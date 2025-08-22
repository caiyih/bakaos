# SPDX-License-Identifier: MPL-2.0
# Minimal hello world program for LoongArch64
# adapted from https://asterinas.github.io/book/ostd/a-100-line-kernel.html

.global _start                      # entry point
.section .text                      # code section
_start:
    li      a7, 64                  # syscall number of write
    li      a0, 1                   # stdout
    la      a1, message             # address of message         
    la      a2, message_end
    sub     a2, a2, a1              # calculate message len
    ecall
    li      a7, 93                  # syscall number of exit, move it to a7
    li      a0, 0                   # exit code, move it to a0
    ecall

.section .rodata                    # read only data section
message:
    .ascii  "Hello, world\n"
message_end:

