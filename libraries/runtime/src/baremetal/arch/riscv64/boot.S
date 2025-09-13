PTE_FLAG = 0xCF
VIRT_ADDR_OFFSET = 0xffffffc000000000

    .section .text.entry
    .global _start
_start:
    la t0, boot_pt

    // get the physical page number of PageTabe
    srli t0, t0, 12 

    // set the mode to SV39
    // TODO: Use SV48 to keep consistency with other architectures
    li t1, 8 << 60
    or t0, t0, t1

    csrw satp, t0
    sfence.vma

    // continue at the high address
    la t0, _start_high
    li t1, VIRT_ADDR_OFFSET
    add t0, t0, t1

    // tail
    jr t0

_start_high:
    xor ra, ra, ra
    xor fp, fp, fp

    la sp, __stack_top
    li t0, VIRT_ADDR_OFFSET
    add sp, t0, sp

    j rust_boot_main

.balign 4096
boot_pt:
    .zero 2 * 8
    .quad (0x80000 << 10) | PTE_FLAG
    .zero 253 * 8
    .quad (0x00000 << 10) | PTE_FLAG
    .quad (0x40000 << 10) | PTE_FLAG
    .quad (0x80000 << 10) | PTE_FLAG
    .zero 252 * 8
    .quad 0
