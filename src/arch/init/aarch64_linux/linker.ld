ENTRY(init_asm)

SECTIONS
{
    kernel_start = .;

    .hdr : {
        KEEP(*(.hdr))
    }
    .text :
    {
        *(.text*)
    }

    .tdata : {
        *(.tdata .tdata.*)
    }
    .tbss : {
        *(.tbss .tbss.*)
    }
    .data.rel.ro :
    {
        *(.data.rel.ro.local .data.rel.ro.local.*)
        *(.data.rel.ro .data.rel.ro.*)
    }
    .dynamic : {
        *(.dynamic)
    }

    .rodata :
    {
        *(.rodata*)
    }

    .data :
    {
        *(.data*)
    }

    .bss :
    {
        bss_start = .;
        *(.bss*)
        stack_bottom = .;
        INCLUDE stack_size.ld
        . = ALIGN(16);
        stack = .;
        bss_end = .;
    }

    kernel_end = .;
}