// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! A semantic versioning implementation, because "1.2.3" is too hard to compare by eye.

use std::fmt;

use crate::core::error::{self, Result};
use crate::core::values::repr_str;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub parts: (u64, u64, u64),
}

fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

impl Version {
    pub fn parse(text: &str) -> Result<Version> {
        let invalid = || error::value_error(format!("not a semantic version: {}", repr_str(text)));
        let pieces: Vec<&str> = text.trim().split('.').collect();
        if pieces.len() != 3 || !pieces.iter().all(|p| all_digits(p)) {
            return Err(invalid());
        }
        let number = |p: &str| p.parse::<u64>().map_err(|_| invalid());
        Ok(Version { parts: (number(pieces[0])?, number(pieces[1])?, number(pieces[2])?) })
    }

    pub fn loose(text: &str) -> Result<Version> {
        let mut parts: Vec<&str> = text.split('.').collect();
        parts.extend(["0", "0"]);
        Version::parse(&parts[..3].join("."))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.parts.0, self.parts.1, self.parts.2)
    }
}

const OPERATORS: [&str; 8] = [">=", "<=", "==", "!=", ">", "<", "^", "~"];

/// Splits a clause into its operator and version, as `^(op)?\s*(\d+(?:\.\d+){0,2})$` would.
fn parse_clause(clause: &str) -> Option<(&str, &str)> {
    let op = OPERATORS.iter().copied().find(|op| clause.starts_with(op)).unwrap_or("");
    let raw = clause[op.len()..].trim_start();
    let pieces: Vec<&str> = raw.split('.').collect();
    (pieces.len() <= 3 && pieces.iter().all(|p| all_digits(p))).then_some((op, raw))
}

/// Check a version against a comma-separated constraint list, e.g. '>=1.2,<2'.
pub fn satisfies(version: &Version, spec: &str) -> Result<bool> {
    for clause in spec.split(',').map(str::trim).filter(|c| !c.is_empty()) {
        let (op, raw) =
            parse_clause(clause).ok_or_else(|| error::value_error(format!("bad constraint: {}", repr_str(clause))))?;
        let target = Version::loose(raw)?;
        let ok = match op {
            "^" => *version >= target && version.parts.0 == target.parts.0,
            "~" => *version >= target && (version.parts.0, version.parts.1) == (target.parts.0, target.parts.1),
            ">=" => *version >= target,
            "<=" => *version <= target,
            "!=" => *version != target,
            ">" => *version > target,
            "<" => *version < target,
            _ => *version == target,
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}
