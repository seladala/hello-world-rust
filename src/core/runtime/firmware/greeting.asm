; Greeting guest. The ROM loads this sector from disk at address 0x2000.

.equ PI_DIGITS,    0x8000
.equ PORT_TTY,     0x01
.equ PORT_STATUS,  0x02
.equ PAYLOAD_LEN,  11
.equ FRAME_LEN,    12
.equ STATUS_GOOD,  0x600D

.org 0x2000
start:
        MOVI  R7, 0                 ; R7 = completed UART interrupts
        MOVI  R6, payload           ; R6 = pointer into encrypted payload
        MOVI  R5, PI_DIGITS         ; R5 = pointer into pi digits
next_char:
        CMPI  R7, PAYLOAD_LEN
        JZ    done
        LDB   R1, [R5]              ; R1 = current pi digit
        CALL  derive_key            ; R0 = key for this character
        LDB   R2, [R6]              ; R2 = encrypted character
        XOR   R2, R0                ; R2 = decrypted character
        OUT   PORT_TTY, R2
        WAIT  uart_interrupt
        INC   R5
        INC   R6
        JMP   next_char
done:
        MOVI  R2, 10                ; Send LF as the final byte of the UART frame.
        OUT   PORT_TTY, R2
        WAIT  uart_interrupt
        CMPI  R7, FRAME_LEN
        JNZ   bad_frame
        MOVI  R0, STATUS_GOOD
        OUT   PORT_STATUS, R0
        HLT
bad_frame:
        MOVI  R0, 0xBAD
        OUT   PORT_STATUS, R0
        HLT

uart_interrupt:
        INC   R7
        IRET

; ---------------------------------------------------------------------
; derive_key: R0 = (R1 * 13 + 37) & 0xFF
; ---------------------------------------------------------------------
derive_key:
        PUSH  R2
        PUSH  R3
        MOV   R2, R1
        MOVI  R3, 13
        CALL  multiply
        ADDI  R0, 37
        MOVI  R2, 0xFF
        AND   R0, R2
        POP   R3
        POP   R2
        RET

; ---------------------------------------------------------------------
; multiply: R0 = R2 * R3   (shift-and-add; a MUL opcode was rejected
;           by the Architecture Board as "too convenient")
; ---------------------------------------------------------------------
multiply:
        PUSH  R2
        PUSH  R3
        PUSH  R4
        MOVI  R0, 0
mul_loop:
        CMPI  R3, 0
        JZ    mul_done
        MOVI  R4, 1
        AND   R4, R3
        JZ    mul_skip
        ADD   R0, R2
mul_skip:
        SHL   R2
        SHR   R3
        JMP   mul_loop
mul_done:
        POP   R4
        POP   R3
        POP   R2
        RET

; ---------------------------------------------------------------------
; Encrypted greeting. Do not read this. Seriously.
; ---------------------------------------------------------------------
payload:
        .byte 0x24, 0x57, 0x35, 0x5E, 0x09, 0xBA, 0x48, 0x1C, 0x14, 0x20, 0x02

.org 0x21FE
        .word 0xAA55                ; Boot sector signature.
