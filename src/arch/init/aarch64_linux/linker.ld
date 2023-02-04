ENTRY(init_asm)

SECTIONS
{
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
        . += 0x1000;
        . = ALIGN(16);
        stack = .;
        . += 16;
        bss_end = .;
    }
}