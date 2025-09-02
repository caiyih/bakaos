# SPDX-License-Identifier: MPL-2.0
# Minimal hello world program for LoongArch64
# adapted from https://asterinas.github.io/book/ostd/a-100-line-kernel.html

.global _start                      # entry point
.section .text                      # code section
_start:
    li.d    $a7, 64                 # syscall number of write
    li.d    $a0, 1                  # stdout
    la.abs  $a1, message            # address of message         
    la.abs  $a2, message_end
    sub.d   $a2, $a2, $a1           # calculate message len
    syscall 0x0
    li.d    $a7, 93                 # syscall number of exit, move it to $a7
    li.d    $a0, 0                  # exit code, move it to $a0
    syscall 0x0

.section .rodata                    # read only data section
message:
    .ascii  "Hello, world\n"
message_end:

