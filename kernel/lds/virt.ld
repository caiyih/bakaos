OUTPUT_ARCH(riscv64gc)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;
VIRTUAL_BASE_ADDRESS = 0xffffffc080200000;

SECTIONS
{
    . = VIRTUAL_BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text ALIGN(4K) : AT(BASE_ADDRESS) {
        *(.text.entry)
        . = ALIGN(4K);
        strampoline = .;
        *(.text.trampoline);
        . = ALIGN(4K);
        *(.text .text.*)
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
        . += 0x00080000;
        . = ALIGN(4K);
        PROVIDE(__tmp_stack_top = .);
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    . = ALIGN(8);

    .debug_line : {
        KEEP(*(.debug_line))
    }

    /*
        /DISCARD/ : {
            *(.eh_frame)
        }
    */

    /* Inserting these two lines */
    . = ALIGN(8);
    /* PROVIDE(__eh_frame = .); */
    /* before .eh_frame rule */
    .eh_frame : { KEEP (*(.eh_frame)) *(.eh_frame.*) }
}
