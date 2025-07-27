OUTPUT_ARCH(riscv)

BASE_ADDRESS = 0x40200000;
VIRTUAL_OFFSET = 0xffffffc000000000;
VIRTUAL_BASE_ADDRESS = BASE_ADDRESS + VIRTUAL_OFFSET;

ENTRY(BASE_ADDRESS)

SECTIONS
{
    . = VIRTUAL_BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text ALIGN(4K) : AT(BASE_ADDRESS) {
        KEEP(*(.text.entry))
        *(.text .text.*)
        . = ALIGN(4K);
        strampoline = .;
        *(.text.trampoline_user);
        *(.text.trampoline_kernel);
    }

    . = ALIGN(4K);
    etext = .;
    srodata = .;
    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data ALIGN(4K) : {
        . = ALIGN(4K);
        *(.data.prepage .data.prepage.*)
        . = ALIGN(4K);
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;
    .bss : {
        *(.bss.heap)
        . += 0x02000000;
        *(.bss.stack)
        PROVIDE(__tmp_stack_bottom = .);
        . = ALIGN(4K);
        . += 0x00180000;
        . = ALIGN(4K);
        PROVIDE(__tmp_stack_top = .);
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;

    . = ALIGN(8);

    .debug_line : {
        KEEP(*(.debug_line))
    }

    . = ALIGN(8);

    ekernel = .;
}
