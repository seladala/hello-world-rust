// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! Dynamically typed values, for payloads that may be anything at all.
//!
//! `repr()` renders a value the way the original interpreter would, so that
//! debug logs stay byte-for-byte comparable with the reference implementation.

use std::fmt::Write;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Dict(Vec<(String, Value)>),
}

impl Value {
    pub fn str(text: impl Into<String>) -> Value {
        Value::Str(text.into())
    }

    pub fn dict<K: Into<String>>(items: impl IntoIterator<Item = (K, Value)>) -> Value {
        Value::Dict(items.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Looks up a key in a dict value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Dict(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Removes a key from a dict value and returns it.
    pub fn pop(&mut self, key: &str) -> Option<Value> {
        match self {
            Value::Dict(items) => {
                let index = items.iter().position(|(k, _)| k == key)?;
                Some(items.remove(index).1)
            }
            _ => None,
        }
    }

    /// `str()` of a value: strings as they are, everything else as its repr.
    pub fn py_str(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            other => other.repr(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::None => "NoneType",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::List(_) => "list",
            Value::Tuple(_) => "tuple",
            Value::Dict(_) => "dict",
        }
    }

    pub fn repr(&self) -> String {
        match self {
            Value::None => "None".to_owned(),
            Value::Bool(true) => "True".to_owned(),
            Value::Bool(false) => "False".to_owned(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => repr_float(*f),
            Value::Str(s) => repr_str(s),
            Value::List(items) => format!("[{}]", join_repr(items)),
            Value::Tuple(items) if items.len() == 1 => format!("({},)", items[0].repr()),
            Value::Tuple(items) => format!("({})", join_repr(items)),
            Value::Dict(items) => {
                let body: Vec<String> = items.iter().map(|(k, v)| format!("{}: {}", repr_str(k), v.repr())).collect();
                format!("{{{}}}", body.join(", "))
            }
        }
    }
}

fn join_repr(items: &[Value]) -> String {
    items.iter().map(Value::repr).collect::<Vec<_>>().join(", ")
}

/// String literal representation: single quotes unless only double quotes avoid escaping.
pub fn repr_str(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') { '"' } else { '\'' };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if is_printable(c) => out.push(c),
            c if (c as u32) < 0x100 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c if (c as u32) < 0x10000 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => {
                let _ = write!(out, "\\U{:08x}", c as u32);
            }
        }
    }
    out.push(quote);
    out
}

/// Printable in the sense of `str.isprintable()`: the space is, other separators and controls are not.
pub fn is_printable(c: char) -> bool {
    c == ' ' || !(c.is_control() || c.is_whitespace() || matches!(c, '\u{ad}' | '\u{200b}'..='\u{200f}'))
}

/// Shortest round-tripping float representation with the original's exponent rules.
pub fn repr_float(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    let magnitude = value.abs();
    if value == 0.0 || (1e-4..1e16).contains(&magnitude) {
        let text = format!("{value}");
        return if text.contains('.') { text } else { format!("{text}.0") };
    }
    let text = format!("{value:e}");
    let (mantissa, exponent) = text.split_once('e').expect("exponent");
    let exponent: i32 = exponent.parse().expect("exponent digits");
    let sign = if exponent < 0 { '-' } else { '+' };
    format!("{mantissa}e{sign}{:02}", exponent.abs())
}

/// Integer literal parsing with automatic base detection, like `int(text, 0)`:
/// `0x`/`0o`/`0b` prefixes, optional sign, single underscores between digits,
/// and no leading zeros on non-zero decimals.
pub fn parse_int_literal(text: &str) -> Option<i64> {
    let text = text.trim();
    let (negative, body) = match text.as_bytes().first()? {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    let lower = body.to_ascii_lowercase();
    let (radix, digits) = match lower.get(..2) {
        Some("0x") => (16, &lower[2..]),
        Some("0o") => (8, &lower[2..]),
        Some("0b") => (2, &lower[2..]),
        _ => (10, lower.as_str()),
    };
    // After a prefix a single leading underscore is allowed ("0x_ff").
    let digits = if radix != 10 { digits.strip_prefix('_').unwrap_or(digits) } else { digits };
    if digits.is_empty() || digits.starts_with('_') || digits.ends_with('_') || digits.contains("__") {
        return None;
    }
    let clean: String = digits.chars().filter(|&c| c != '_').collect();
    if radix == 10 && clean.len() > 1 && clean.starts_with('0') && clean.chars().any(|c| c != '0') {
        return None;
    }
    let magnitude = i128::from_str_radix(&clean, radix).ok()?;
    i64::try_from(if negative { -magnitude } else { magnitude }).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_literals() {
        assert_eq!(parse_int_literal("0x8000"), Some(0x8000));
        assert_eq!(parse_int_literal(" 1_000 "), Some(1000));
        assert_eq!(parse_int_literal("-0b101"), Some(-5));
        assert_eq!(parse_int_literal("00"), Some(0));
        assert_eq!(parse_int_literal("010"), None);
        assert_eq!(parse_int_literal("R1"), None);
        assert_eq!(parse_int_literal("1__0"), None);
    }

    #[test]
    fn reprs() {
        assert_eq!(Value::Tuple(vec![Value::Int(0), Value::str("h")]).repr(), "(0, 'h')");
        assert_eq!(repr_str("it's"), "\"it's\"");
        assert_eq!(repr_float(0.0004435619301083008), "0.0004435619301083008");
        assert_eq!(repr_float(1e-5), "1e-05");
        assert_eq!(repr_float(1e16), "1e+16");
        assert_eq!(repr_float(2.0), "2.0");
        assert_eq!(
            Value::dict([("a", Value::Int(1)), ("b", Value::List(vec![Value::None]))]).repr(),
            "{'a': 1, 'b': [None]}"
        );
    }
}
