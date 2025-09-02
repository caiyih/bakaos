OUTPUT_ARCH(loongarch64)
ENTRY(_start)
BASE_ADDRESS = 0x80000000;
VIRTUAL_BASE_ADDRESS = 0x9000000080000000;

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
        . = ALIGN(4K);
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
        . += 0x00800000;
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
