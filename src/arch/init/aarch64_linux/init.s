.macro adrl rd, symbol
    adrp \rd, \symbol
    add  \rd, \rd, :lo12:\symbol
.endm

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
    adrl x3, stack
    mov sp, x3
    mov x19, x0

    mov x0, #0
    adrl x1, bss_start
    adrl x2, bss_end
.loop:
    str x0, [x1]
    add x1, x1, #8
    cmp x1, x2
    b.lt .loop

    sub x0, x30, #4 /* get the base address */
    adrl x1, _DYNAMIC
    bl relocate

    mov x0, x19
    b init
.popsection
