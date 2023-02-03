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
    sub x1, x30, #4 /* get the base address */
    adr x3, stack
    mov sp, x3
    adr x2, _DYNAMIC
    b init
.popsection

.pushsection .bss
stack:
    .skip 0x1000
.popsection
