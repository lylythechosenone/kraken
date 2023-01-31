SECTIONS
{
    .hdr ALIGN(4K):
    {
        KEEP(*(.hdr))
    }
    .text :
    {
        KEEP(*(.text*))
    }

    .tdata ALIGN(4K): {
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

    .rodata ALIGN(4K):
    {
        KEEP(*(.rodata*))
    }

    .data ALIGN(4K):
    {
        KEEP(*(.data*))
    }

    .bss ALIGN(4K):
    {
        KEEP(*(.bss*))
    }
}