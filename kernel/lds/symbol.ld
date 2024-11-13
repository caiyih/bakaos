/*
    This linker script is exactly same as the one used for producing the kernel image.
    The only differencee is that the base address is changed to VIRTUAL_BASE_ADDRESS.
    This is intentional to produce the symbol table for the kernel image.
    As the kernel enabled virtual memory that mapped all the physical memory to the higher half,
    the symbol table is produced with the virtual address.
    You can see comments in main.rs to learn more about the virtual memory design.
*/
OUTPUT_ARCH(riscv64gc)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;
VIRTUAL_BASE_ADDRESS = 0xffffffc080200000;

SECTIONS
{
    . = VIRTUAL_BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
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
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
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

    /*
        /DISCARD/ : {
            *(.eh_frame)
        }
    */

    /* Inserting these two lines */
    . = ALIGN(8);
    PROVIDE(__eh_frame = .);
    /* before .eh_frame rule */
    .eh_frame : { KEEP (*(.eh_frame)) *(.eh_frame.*) }
}
