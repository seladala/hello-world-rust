//! UHX-16 cycle-approximate emulator.
//!
//! 64 KiB of byte-addressable memory, eight 16-bit registers, a downward
//! growing stack and three flags (Z, N, C). I/O happens through numbered ports
//! which the host wires up to closures.

use std::collections::HashMap;

use super::opcodes::Op;
use crate::core::error::{self, Result};

pub const MASK16: i64 = 0xFFFF;

pub type OutputSink = Box<dyn FnMut(u16)>;
pub type InputSource = Box<dyn FnMut() -> i64>;
type Handler = fn(&mut Machine) -> Result<()>;

pub struct Machine {
    pub memory: Vec<u8>,
    pub registers: [u16; 8],
    pub pc: u16,
    pub sp: u16,
    pub zero: bool,
    pub negative: bool,
    pub carry: bool,
    pub halted: bool,
    pub cycles: u64,
    pub out_ports: HashMap<u8, OutputSink>,
    pub in_ports: HashMap<u8, InputSource>,
    dispatch: HashMap<Op, Handler>,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub const MEMORY_SIZE: usize = 0x10000;

    pub fn new() -> Machine {
        Machine {
            memory: vec![0; Self::MEMORY_SIZE],
            registers: [0; 8],
            pc: 0,
            sp: 0xFFFE,
            zero: false,
            negative: false,
            carry: false,
            halted: false,
            cycles: 0,
            out_ports: HashMap::new(),
            in_ports: HashMap::new(),
            dispatch: Op::ALL.iter().map(|&op| (op, Self::handler(op))).collect(),
        }
    }

    // ------------------------------------------------------------------ setup
    pub fn load(&mut self, image: &[u8], origin: usize) -> Result<()> {
        if origin + image.len() > Self::MEMORY_SIZE {
            return Err(error::machine_fault("image does not fit in memory"));
        }
        self.memory[origin..origin + image.len()].copy_from_slice(image);
        Ok(())
    }

    pub fn attach_output(&mut self, port: u8, sink: impl FnMut(u16) + 'static) {
        self.out_ports.insert(port, Box::new(sink));
    }

    pub fn attach_input(&mut self, port: u8, source: impl FnMut() -> i64 + 'static) {
        self.in_ports.insert(port, Box::new(source));
    }

    // ---------------------------------------------------------------- helpers
    fn fetch8(&mut self) -> u8 {
        let value = self.memory[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
        value
    }

    fn fetch16(&mut self) -> u16 {
        let lo = self.fetch8() as u16;
        lo | ((self.fetch8() as u16) << 8)
    }

    fn register(&self, index: u8) -> Result<u16> {
        self.registers.get(index as usize).copied().ok_or_else(|| error::index_error("list index out of range"))
    }

    fn set_register(&mut self, index: u8, value: u16) -> Result<()> {
        let slot = self
            .registers
            .get_mut(index as usize)
            .ok_or_else(|| error::index_error("list assignment index out of range"))?;
        *slot = value;
        Ok(())
    }

    fn set_flags(&mut self, raw: i64) -> u16 {
        let result = (raw & MASK16) as u16;
        self.zero = result == 0;
        self.negative = result & 0x8000 != 0;
        self.carry = !(0..=MASK16).contains(&raw);
        result
    }

    fn push(&mut self, value: u16) -> Result<()> {
        self.sp = self.sp.wrapping_sub(2);
        let sp = self.sp as usize;
        if sp + 1 >= Self::MEMORY_SIZE {
            return Err(error::index_error("bytearray index out of range"));
        }
        self.memory[sp] = (value & 0xFF) as u8;
        self.memory[sp + 1] = (value >> 8) as u8;
        Ok(())
    }

    fn pop(&mut self) -> Result<u16> {
        let sp = self.sp as usize;
        if sp + 1 >= Self::MEMORY_SIZE {
            return Err(error::index_error("bytearray index out of range"));
        }
        let value = self.memory[sp] as u16 | ((self.memory[sp + 1] as u16) << 8);
        self.sp = self.sp.wrapping_add(2);
        Ok(value)
    }

    // -------------------------------------------------------------- execution
    pub fn step(&mut self) -> Result<()> {
        let opcode = self.fetch8();
        let Some(op) = Op::from_byte(opcode) else {
            return Err(error::machine_fault(format!(
                "illegal opcode 0x{opcode:02X} at 0x{:04X}",
                self.pc.wrapping_sub(1)
            )));
        };
        let handler = self.dispatch[&op];
        handler(self)?;
        self.cycles += op.spec().size() as u64;
        Ok(())
    }

    pub fn run(&mut self, max_cycles: u64) -> Result<u64> {
        while !self.halted {
            if self.cycles > max_cycles {
                return Err(error::machine_fault("watchdog: program refuses to halt"));
            }
            self.step()?;
        }
        Ok(self.cycles)
    }

    // ----------------------------------------------------------- instructions
    fn handler(op: Op) -> Handler {
        match op {
            Op::NOP => Self::op_nop,
            Op::HLT => Self::op_hlt,
            Op::MOVI => Self::op_movi,
            Op::MOV => Self::op_mov,
            Op::LDB => Self::op_ldb,
            Op::STB => Self::op_stb,
            Op::ADD => Self::op_add,
            Op::SUB => Self::op_sub,
            Op::AND => Self::op_and,
            Op::OR => Self::op_or,
            Op::XOR => Self::op_xor,
            Op::ADDI => Self::op_addi,
            Op::CMP => Self::op_cmp,
            Op::CMPI => Self::op_cmpi,
            Op::SHL => Self::op_shl,
            Op::SHR => Self::op_shr,
            Op::INC => Self::op_inc,
            Op::DEC => Self::op_dec,
            Op::JMP => Self::op_jmp,
            Op::JZ => Self::op_jz,
            Op::JNZ => Self::op_jnz,
            Op::JC => Self::op_jc,
            Op::JNC => Self::op_jnc,
            Op::CALL => Self::op_call,
            Op::RET => Self::op_ret,
            Op::PUSH => Self::op_push,
            Op::POP => Self::op_pop,
            Op::OUT => Self::op_out,
            Op::IN => Self::op_in,
        }
    }

    pub fn op_nop(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn op_hlt(&mut self) -> Result<()> {
        self.halted = true;
        Ok(())
    }

    pub fn op_movi(&mut self) -> Result<()> {
        let rd = self.fetch8();
        let value = self.fetch16();
        self.set_register(rd, value)
    }

    pub fn op_mov(&mut self) -> Result<()> {
        let (rd, rs) = (self.fetch8(), self.fetch8());
        let value = self.register(rs)?;
        self.set_register(rd, value)
    }

    pub fn op_ldb(&mut self) -> Result<()> {
        let (rd, rs) = (self.fetch8(), self.fetch8());
        let value = self.memory[self.register(rs)? as usize] as u16;
        self.set_register(rd, value)
    }

    pub fn op_stb(&mut self) -> Result<()> {
        let (rd, rs) = (self.fetch8(), self.fetch8());
        let address = self.register(rd)? as usize;
        self.memory[address] = (self.register(rs)? & 0xFF) as u8;
        Ok(())
    }

    fn alu(&mut self, f: fn(i64, i64) -> i64, store: bool) -> Result<()> {
        let (rd, rs) = (self.fetch8(), self.fetch8());
        let raw = f(self.register(rd)? as i64, self.register(rs)? as i64);
        let result = self.set_flags(raw);
        if store {
            self.set_register(rd, result)?;
        }
        Ok(())
    }

    fn alu_imm(&mut self, f: fn(i64, i64) -> i64, store: bool) -> Result<()> {
        let rd = self.fetch8();
        let current = self.register(rd)? as i64;
        let raw = f(current, self.fetch16() as i64);
        let result = self.set_flags(raw);
        if store {
            self.set_register(rd, result)?;
        }
        Ok(())
    }

    pub fn op_add(&mut self) -> Result<()> {
        self.alu(|a, b| a + b, true)
    }

    pub fn op_sub(&mut self) -> Result<()> {
        self.alu(|a, b| a - b, true)
    }

    pub fn op_and(&mut self) -> Result<()> {
        self.alu(|a, b| a & b, true)
    }

    pub fn op_or(&mut self) -> Result<()> {
        self.alu(|a, b| a | b, true)
    }

    pub fn op_xor(&mut self) -> Result<()> {
        self.alu(|a, b| a ^ b, true)
    }

    pub fn op_addi(&mut self) -> Result<()> {
        self.alu_imm(|a, b| a + b, true)
    }

    pub fn op_cmp(&mut self) -> Result<()> {
        self.alu(|a, b| a - b, false)
    }

    pub fn op_cmpi(&mut self) -> Result<()> {
        self.alu_imm(|a, b| a - b, false)
    }

    fn unary(&mut self, f: fn(i64) -> i64) -> Result<()> {
        let rd = self.fetch8();
        let raw = f(self.register(rd)? as i64);
        let result = self.set_flags(raw);
        self.set_register(rd, result)
    }

    pub fn op_shl(&mut self) -> Result<()> {
        self.unary(|a| a << 1)
    }

    pub fn op_shr(&mut self) -> Result<()> {
        self.unary(|a| a >> 1)
    }

    pub fn op_inc(&mut self) -> Result<()> {
        self.unary(|a| a + 1)
    }

    pub fn op_dec(&mut self) -> Result<()> {
        self.unary(|a| a - 1)
    }

    fn jump_if(&mut self, condition: bool) -> Result<()> {
        let target = self.fetch16();
        if condition {
            self.pc = target;
        }
        Ok(())
    }

    pub fn op_jmp(&mut self) -> Result<()> {
        self.jump_if(true)
    }

    pub fn op_jz(&mut self) -> Result<()> {
        self.jump_if(self.zero)
    }

    pub fn op_jnz(&mut self) -> Result<()> {
        self.jump_if(!self.zero)
    }

    pub fn op_jc(&mut self) -> Result<()> {
        self.jump_if(self.carry)
    }

    pub fn op_jnc(&mut self) -> Result<()> {
        self.jump_if(!self.carry)
    }

    pub fn op_call(&mut self) -> Result<()> {
        let target = self.fetch16();
        self.push(self.pc)?;
        self.pc = target;
        Ok(())
    }

    pub fn op_ret(&mut self) -> Result<()> {
        self.pc = self.pop()?;
        Ok(())
    }

    pub fn op_push(&mut self) -> Result<()> {
        let rs = self.fetch8();
        let value = self.register(rs)?;
        self.push(value)
    }

    pub fn op_pop(&mut self) -> Result<()> {
        let rd = self.fetch8();
        let value = self.pop()?;
        self.set_register(rd, value)
    }

    pub fn op_out(&mut self) -> Result<()> {
        let (port, rs) = (self.fetch8(), self.fetch8());
        let value = self.register(rs)?;
        let sink = self
            .out_ports
            .get_mut(&port)
            .ok_or_else(|| error::machine_fault(format!("write to unmapped port 0x{port:02X}")))?;
        sink(value);
        Ok(())
    }

    pub fn op_in(&mut self) -> Result<()> {
        let (rd, port) = (self.fetch8(), self.fetch8());
        let source = self
            .in_ports
            .get_mut(&port)
            .ok_or_else(|| error::machine_fault(format!("read from unmapped port 0x{port:02X}")))?;
        let value = (source() & MASK16) as u16;
        self.set_register(rd, value)
    }
}
