; НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
; Каждая строка набрана лично на клавиатуре. Честно 🤝✅
; =====================================================================
;  SELFTEST.ASM - Power-On Self Test for the UHX-16
;
;  1. Computes fib(20) and reports it on PORT_FIB (expected: 6765).
;  2. Writes a walking pattern to RAM_TEST..RAM_TEST+255, reads it back
;     and reports the number of mismatches on PORT_RAM (expected: 0).
;  Nobody has ever seen this test fail. It runs anyway.
; =====================================================================

.equ PORT_FIB,  0x10
.equ PORT_RAM,  0x11
.equ RAM_TEST,  0x4000
.equ RAM_SIZE,  256

.org 0x0000
        MOVI  R0, 0                 ; fib(n-2)
        MOVI  R1, 1                 ; fib(n-1)
        MOVI  R2, 19                ; iterations left
fib_loop:
        MOV   R3, R0
        ADD   R3, R1
        MOV   R0, R1
        MOV   R1, R3
        DEC   R2
        JNZ   fib_loop
        OUT   PORT_FIB, R1

        MOVI  R4, RAM_TEST          ; write pattern: mem[addr] = addr low byte ^ 0xA5
        MOVI  R5, RAM_SIZE
        MOVI  R6, 0xA5
ram_write:
        MOV   R0, R4
        XOR   R0, R6
        STB   [R4], R0
        INC   R4
        DEC   R5
        JNZ   ram_write

        MOVI  R4, RAM_TEST
        MOVI  R5, RAM_SIZE
        MOVI  R7, 0                 ; error counter
ram_verify:
        LDB   R1, [R4]
        MOV   R0, R4
        XOR   R0, R6
        MOVI  R2, 0xFF
        AND   R0, R2
        CMP   R0, R1
        JZ    ram_ok
        INC   R7
ram_ok:
        INC   R4
        DEC   R5
        JNZ   ram_verify
        OUT   PORT_RAM, R7
        HLT
