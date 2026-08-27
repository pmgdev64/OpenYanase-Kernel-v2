ENTRY(_start)

SECTIONS
{
    . = 1M;

    .boot :
    {
        KEEP(*(.multiboot))
    }

    .text :
    {
        *(.text .text.*)
    }

    .rodata :
    {
        *(.rodata .rodata.*)
    }

    .data :
    {
        *(.data .data.*)
    }

    .bss :
    {
        stack_bottom = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(16);
        . += 64K;
        stack_top = .;
    }

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.note.*)
    }
}