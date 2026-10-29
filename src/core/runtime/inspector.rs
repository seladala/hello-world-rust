//! Disassembler, so that the machine code can be reviewed by the Architecture Board.

use super::opcodes::Op;

pub fn disassemble(image: &[u8]) -> Vec<String> {
    disassemble_range(image, 0, image.len())
}

pub fn disassemble_range(image: &[u8], start: usize, end: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut pc = start;
    while pc < end {
        let opcode = image[pc];
        let spec = Op::from_byte(opcode).map(Op::spec);
        let Some(spec) = spec.filter(|spec| pc + spec.size() <= end) else {
            lines.push(format!("{pc:04X}:  {opcode:02X}            .byte 0x{opcode:02X}"));
            pc += 1;
            continue;
        };
        let raw: Vec<String> = image[pc..pc + spec.size()].iter().map(|b| format!("{b:02X}")).collect();
        let mut cursor = pc + 1;
        let mut parts = Vec::new();
        for kind in spec.operands.chars() {
            match kind {
                'r' => {
                    parts.push(format!("R{}", image[cursor]));
                    cursor += 1;
                }
                'p' => {
                    parts.push(format!("0x{:02X}", image[cursor]));
                    cursor += 1;
                }
                _ => {
                    parts.push(format!("0x{:04X}", image[cursor] as u16 | (image[cursor + 1] as u16) << 8));
                    cursor += 2;
                }
            }
        }
        let line = format!("{pc:04X}:  {:<14}{} {}", raw.join(" "), spec.mnemonic(), parts.join(", "));
        lines.push(line.trim_end().to_owned());
        pc += spec.size();
    }
    lines
}
