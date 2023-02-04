.pushsection .hdr,"ax",@progbits
bl init_asm
nop
.8byte 0
.8byte 0
.8byte 0
.8byte 0
.8byte 0
.8byte 0
.ascii "ARMd"
.4byte 0

init_asm:
    adr x3, stack
    mov sp, x30
    str x0, [sp, #-8]

    mov x0, #0
    adr x1, bss_start
    adr x2, bss_end
.loop:
    str x0, [x1]
    add x1, x1, #8
    cmp x1, x2
    b.lt .loop

    sub x0, x30, #4 /* get the base address */
    adr x1, _DYNAMIC
    bl relocate

    ldr x0, [sp, #-8]
    sub sp, sp, #0x8
    b init
.popsection
