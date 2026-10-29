; НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
; Каждая строка набрана лично на клавиатуре. Честно 🤝✅
; =====================================================================
;  INIT.ASM - Hello World Firmware, revision 6.7
;  Target: UHX-16 (Unified Hardware eXecution unit)
;
;  Host contract:
;    * decimal digits of pi are preloaded at PI_DIGITS, one digit per byte
;    * port PORT_TTY receives output characters (low byte of the register)
;    * port PORT_STATUS receives a status word right before HLT
;
;  The greeting is stored encrypted with a key schedule derived from pi:
;      key[i] = (pi_digit[i] * 13 + 37) & 0xFF
;  so if anybody ever changes the value of pi, this program will
;  (correctly) stop greeting the world.
; =====================================================================

.equ PI_DIGITS,    0x8000
.equ PORT_TTY,     0x01
.equ PORT_STATUS,  0x02
.equ PAYLOAD_LEN,  11
.equ STATUS_GOOD,  0x600D

.org 0x0000
start:
        MOVI  R7, 0                 ; R7 = character index
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
        INC   R5
        INC   R6
        INC   R7
        JMP   next_char
done:
        MOVI  R0, STATUS_GOOD
        OUT   PORT_STATUS, R0
        HLT

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
