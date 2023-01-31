.pushsection .hdr
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
    ldr x5, =0xDEADBEEF
    sub x1, x30, #4 /* get the base address */
    adr x3, stack
    mov sp, x3
    /* adr x2, _DYNAMIC
    b init */

stack:
    .space 0x1000
.popsection
