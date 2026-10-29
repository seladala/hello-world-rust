//! Two-pass assembler for UHX-16 assembly.
//!
//! Syntax:
//!     label:                  define a label
//!     MNEMONIC op, op         instruction (registers R0..R7, [Rn] allowed for LDB/STB)
//!     .org  ADDR              move the location counter forward
//!     .equ  NAME, VALUE       define a constant
//!     .byte v, v, ...         emit bytes
//!     .word v, v, ...         emit 16-bit little-endian words
//!     ; comment
//!
//! Values may be decimal, 0x-hex, 0b-binary, or a symbol name.
//! String literals are deliberately unsupported: typing "hello world" would be cheating.

use std::collections::HashMap;

use super::opcodes::{operand_size, spec_for_mnemonic, REGISTER_COUNT};
use crate::core::error::{self, Result};
use crate::core::values::{parse_int_literal, repr_str};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssembledProgram {
    pub image: Vec<u8>,
    pub symbols: HashMap<String, i64>,
    pub listing: Vec<(usize, String)>,
}

/// Matches `^([A-Za-z_][A-Za-z0-9_]*):` and returns the label and the remainder after the colon.
fn match_label(line: &str) -> Option<(&str, &str)> {
    let mut chars = line.char_indices();
    let (_, first) = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    for (index, c) in chars {
        if c == ':' {
            return Some((&line[..index], &line[index + 1..]));
        }
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
    }
    None
}

fn split_operands(text: &str) -> Vec<&str> {
    text.split(',').map(str::trim).filter(|part| !part.is_empty()).collect()
}

/// `str.partition(" ")`: split at the first space only.
fn partition(line: &str) -> (&str, &str) {
    line.split_once(' ').unwrap_or((line, ""))
}

fn parse_register(token: &str, line_no: usize) -> Result<u8> {
    let bad = || error::assembly_error(format!("line {line_no}: expected register, got {}", repr_str(token)));
    let inner = token.strip_prefix('[').unwrap_or(token);
    let inner = inner.strip_suffix(']').unwrap_or(inner);
    let mut chars = inner.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some('R' | 'r'), Some(digit @ '0'..='9'), None) => {
            let index = digit as usize - '0' as usize;
            if index >= REGISTER_COUNT {
                return Err(bad());
            }
            Ok(index as u8)
        }
        _ => Err(bad()),
    }
}

fn parse_value(token: &str, symbols: &HashMap<String, i64>, line_no: usize, strict: bool) -> Result<i64> {
    let token = token.trim();
    if let Some(value) = parse_int_literal(token) {
        return Ok(value);
    }
    if let Some(&value) = symbols.get(token) {
        return Ok(value);
    }
    if strict {
        return Err(error::assembly_error(format!("line {line_no}: unknown symbol {}", repr_str(token))));
    }
    Ok(0)
}

fn clean_lines(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, raw)| {
            let line = raw.split(';').next().unwrap_or("").trim();
            (!line.is_empty()).then(|| (index + 1, line.to_owned()))
        })
        .collect()
}

/// Return (new_location, emitted_size) for a label-free line.
fn size_of(line: &str, symbols: &HashMap<String, i64>, line_no: usize, location: i64) -> Result<(i64, i64)> {
    let (head, rest) = partition(line);
    let head = head.to_uppercase();
    let operands = split_operands(rest);
    match head.as_str() {
        ".ORG" => {
            let token = operands.first().ok_or_else(|| error::index_error("list index out of range"))?;
            let target = parse_value(token, symbols, line_no, true)?;
            if target < location {
                return Err(error::assembly_error(format!("line {line_no}: .org moves backwards")));
            }
            Ok((target, 0))
        }
        ".EQU" => Ok((location, 0)),
        ".BYTE" => Ok((location + operands.len() as i64, operands.len() as i64)),
        ".WORD" => Ok((location + 2 * operands.len() as i64, 2 * operands.len() as i64)),
        _ => {
            let spec = spec_for_mnemonic(&head).ok_or_else(|| {
                error::assembly_error(format!("line {line_no}: unknown mnemonic {}", repr_str(&head)))
            })?;
            Ok((location + spec.size() as i64, spec.size() as i64))
        }
    }
}

fn strip_labels(mut line: &str) -> &str {
    while let Some((_, rest)) = match_label(line) {
        line = rest.trim();
    }
    line
}

pub fn assemble(source: &str) -> Result<AssembledProgram> {
    let lines = clean_lines(source);
    let mut symbols: HashMap<String, i64> = HashMap::new();

    // Pass 1: assign addresses to labels and constants.
    let mut location: i64 = 0;
    for (line_no, line) in &lines {
        let line_no = *line_no;
        let mut line = line.as_str();
        while let Some((name, rest)) = match_label(line) {
            if symbols.contains_key(name) {
                return Err(error::assembly_error(format!("line {line_no}: duplicate symbol {}", repr_str(name))));
            }
            symbols.insert(name.to_owned(), location);
            line = rest.trim();
        }
        if line.is_empty() {
            continue;
        }
        let (head, rest) = partition(line);
        if head.to_uppercase() == ".EQU" {
            let operands = split_operands(rest);
            let [name, value] = operands[..] else {
                return Err(error::value_error(format!(
                    "line {line_no}: .equ expects exactly 2 operands, got {}",
                    operands.len()
                )));
            };
            let value = parse_value(value, &symbols, line_no, true)?;
            symbols.insert(name.to_owned(), value);
            continue;
        }
        location = size_of(line, &symbols, line_no, location)?.0;
    }

    // Pass 2: emit bytes.
    let mut image: Vec<u8> = Vec::new();
    let mut listing: Vec<(usize, String)> = Vec::new();
    for (line_no, line) in &lines {
        let line_no = *line_no;
        let line = strip_labels(line);
        if line.is_empty() {
            continue;
        }
        let (head, rest) = partition(line);
        let head = head.to_uppercase();
        let operands = split_operands(rest);
        match head.as_str() {
            ".EQU" => continue,
            ".ORG" => {
                let token = operands.first().ok_or_else(|| error::index_error("list index out of range"))?;
                let target = parse_value(token, &symbols, line_no, true)?;
                if target > image.len() as i64 {
                    image.resize(target as usize, 0);
                }
                continue;
            }
            _ => {}
        }
        listing.push((image.len(), line.to_owned()));
        match head.as_str() {
            ".BYTE" => {
                for token in operands {
                    let value = parse_value(token, &symbols, line_no, true)?;
                    if !(0..=0xFF).contains(&value) {
                        return Err(error::assembly_error(format!("line {line_no}: byte out of range: {value}")));
                    }
                    image.push(value as u8);
                }
            }
            ".WORD" => {
                for token in operands {
                    let value = (parse_value(token, &symbols, line_no, true)? & 0xFFFF) as u16;
                    image.extend(value.to_le_bytes());
                }
            }
            _ => {
                let spec = spec_for_mnemonic(&head).ok_or_else(|| error::key_error(repr_str(&head)))?;
                let expected = spec.operands.chars().count();
                if operands.len() != expected {
                    return Err(error::assembly_error(format!(
                        "line {line_no}: {head} expects {expected} operands, got {}",
                        operands.len()
                    )));
                }
                image.push(spec.op as u8);
                for (kind, token) in spec.operands.chars().zip(operands) {
                    if kind == 'r' {
                        image.push(parse_register(token, line_no)?);
                    } else {
                        let value = parse_value(token, &symbols, line_no, true)?;
                        let size = operand_size(kind);
                        let masked = value & ((1i64 << (8 * size)) - 1);
                        image.extend(&masked.to_le_bytes()[..size]);
                    }
                }
            }
        }
    }
    Ok(AssembledProgram { image, symbols, listing })
}
