; UHX-16 boot ROM. Load sector zero from the read-only disk into RAM.

.equ PORT_STATUS,  0x02
.equ DISK_LBA,     0x20
.equ DISK_DATA,    0x21
.equ DISK_EOF,     0x100
.equ BOOT_ADDRESS, 0x2000
.equ SECTOR_SIZE,  512
.equ SIGNATURE,    0x21FE

.org 0
        MOVI  R0, 0
        OUT   DISK_LBA, R0
        MOVI  R1, BOOT_ADDRESS
        MOVI  R2, SECTOR_SIZE
read_sector:
        IN    R0, DISK_DATA
        CMPI  R0, DISK_EOF
        JZ    boot_failed
        STB   [R1], R0
        INC   R1
        DEC   R2
        JNZ   read_sector

        MOVI  R1, SIGNATURE
        LDB   R0, [R1]
        CMPI  R0, 0x55
        JNZ   boot_failed
        INC   R1
        LDB   R0, [R1]
        CMPI  R0, 0xAA
        JNZ   boot_failed
        JMP   BOOT_ADDRESS

boot_failed:
        MOVI  R0, 0xBAD
        OUT   PORT_STATUS, R0
        HLT
