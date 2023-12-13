.section "text"

.define RAM_START       0x1000000
.define RAM_STACK_START 0x1010000
.define RAM_STACK_END   0x1001000

.define STDIO_WRT 33554432

start:
    loadid RAM_STACK_START sp ; setup stack ; 05  00 00 01 01  13
    
    movrd bp sp ; bp <- sp ; 12  14  13

    ; Prepare message
    loadid message r0 ; 05  00 01 00 00 00
;    call print_msg ; 0B  45 00 00 00
    
    call msg_len ; 0B  15 00 00 00

    halt ; 01

; Takes ptr in r0
; Returns result in r1
msg_len:
    push sp ; 0F 13
    push bp ; 0F 14
    movrd bp sp ; 12 14 13
    
    push r2 ; 0F 02

    loadid 0 r1 ; 05  00 00 00 00  01
    loadib 0 r20l ; checks characters ; 08  00 00 00 00  08 ; ERROR

    @lenloop:
    ; if r20l == 0, jmp -> @loopend
    ldptrb r0 r20l
    icmpub 0 r20l
    jrc msg_len@loopend ZR
    
    ; i++
    iadd 1 r1

    jmp msg_len@lenloop

    @loopend:
    pop r2 ; restore r2

    pop bp
    pop sp
    ret

; Takes ptr to null-term str in r0
print_msg:
    push sp
    push bp
    movrd bp sp
    
    push r1 ; save r1
    
    push r0
    call msg_len ; get length in r1
    pop r0

    push r4 ; preserve r4
    
    @loop:
    ; while len != 0
    icmpud 0 r1
    jrc print_msg@loop_end ZR

    ldptrb r0 r40l
    call putc ; print character at r40l
    
    isub 1 r1 ; r1 -= 1

    jmp print_msg@loop

    @loop_end:
    pop r4 ; restore r4

    pop r1

    pop bp
    pop sp

    ret

; takes char in r40l
putc:
    push sp
    push bp
    movrd bp sp

    ; store at STDIO_WRT

    push r0

    loadid STDIO_WRT r0
    stptrb r40l r0; store r40l at STDIO_WRT

    pop r0

    pop bp
    pop sp
    ret

panic:
    nop
    halt

.section "data"

.section "rodata"
message: .db "Hello, world!" 0x0A 0x00

;.section "ints" ; interrupt handlers

