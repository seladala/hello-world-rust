//! UHX-16 Instruction Set Architecture.
//!
//! The Unified Hardware eXecution unit is a 16-bit, 8-register, little-endian
//! CPU designed by a committee whose only requirement was "it must be able to
//! say hello world, eventually".
//!
//! Operand kinds:
//!     'r' - register index, 1 byte (R0..R7)
//!     'i' - 16-bit immediate / address, 2 bytes little-endian
//!     'p' - 8-bit I/O port number, 1 byte

macro_rules! instruction_set {
    ($($name:ident = $code:literal, $operands:literal, $description:literal;)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum Op {
            $($name = $code,)*
        }

        impl Op {
            pub const ALL: &'static [Op] = &[$(Op::$name,)*];

            pub fn from_byte(byte: u8) -> Option<Op> {
                match byte {
                    $($code => Some(Op::$name),)*
                    _ => None,
                }
            }

            pub fn name(self) -> &'static str {
                match self {
                    $(Op::$name => stringify!($name),)*
                }
            }

            pub fn spec(self) -> InstructionSpec {
                match self {
                    $(Op::$name => InstructionSpec { op: Op::$name, operands: $operands, description: $description },)*
                }
            }
        }
    };
}

instruction_set! {
    NOP = 0x00, "", "do nothing, but professionally";
    HLT = 0x01, "", "stop the world";

    MOVI = 0x10, "ri", "rd <- imm16";
    MOV = 0x11, "rr", "rd <- rs";
    LDB = 0x12, "rr", "rd <- mem8[rs]";
    STB = 0x13, "rr", "mem8[rd] <- rs & 0xFF";

    ADD = 0x20, "rr", "rd <- rd + rs";
    SUB = 0x21, "rr", "rd <- rd - rs";
    AND = 0x22, "rr", "rd <- rd & rs";
    OR = 0x23, "rr", "rd <- rd | rs";
    XOR = 0x24, "rr", "rd <- rd ^ rs";
    ADDI = 0x25, "ri", "rd <- rd + imm16";
    CMP = 0x26, "rr", "flags <- rd - rs";
    CMPI = 0x27, "ri", "flags <- rd - imm16";
    SHL = 0x28, "r", "rd <- rd << 1";
    SHR = 0x29, "r", "rd <- rd >> 1";
    INC = 0x2A, "r", "rd <- rd + 1";
    DEC = 0x2B, "r", "rd <- rd - 1";

    JMP = 0x30, "i", "pc <- addr";
    JZ = 0x31, "i", "if Z: pc <- addr";
    JNZ = 0x32, "i", "if !Z: pc <- addr";
    JC = 0x33, "i", "if C: pc <- addr";
    JNC = 0x34, "i", "if !C: pc <- addr";

    CALL = 0x38, "i", "push pc; pc <- addr";
    RET = 0x39, "", "pc <- pop";
    PUSH = 0x3A, "r", "push rs";
    POP = 0x3B, "r", "rd <- pop";

    OUT = 0x40, "pr", "port <- rs";
    IN = 0x41, "rp", "rd <- port";
}

pub const REGISTER_COUNT: usize = 8;

pub fn operand_size(kind: char) -> usize {
    match kind {
        'r' | 'p' => 1,
        'i' => 2,
        other => panic!("unknown operand kind {other:?}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionSpec {
    pub op: Op,
    pub operands: &'static str,
    pub description: &'static str,
}

impl InstructionSpec {
    pub fn mnemonic(&self) -> &'static str {
        self.op.name()
    }

    pub fn size(&self) -> usize {
        1 + self.operands.chars().map(operand_size).sum::<usize>()
    }
}

pub fn spec_for_mnemonic(mnemonic: &str) -> Option<InstructionSpec> {
    Op::ALL.iter().map(|op| op.spec()).find(|spec| spec.mnemonic() == mnemonic)
}
