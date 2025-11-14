.section "text"

; Should always start at 0x00000000
start:
    dsin
    loadid 0x5153 r0

    halt

.section "data"

.section "rodata"
msg_not_bootable:
    .db "Not a bootable drive!" 0x0A 0x00

.section "interrupts"

