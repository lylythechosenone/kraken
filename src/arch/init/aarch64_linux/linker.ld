SECTIONS
{
    .hdr : {
        *(.hdr)
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
        *(.bss*)
    }
}