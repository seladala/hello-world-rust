// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! Strongly-typed-ish configuration facade over a perfectly ordinary .ini file.

use std::path::{Path, PathBuf};

use super::container::Service;
use super::error::{self, Result};
use super::values::{parse_int_literal, repr_str};

pub fn default_path() -> PathBuf {
    super::package_dir().join("settings.ini")
}

/// A faithful-enough reimplementation of a classic INI parser: `[sections]`, `key = value`
/// or `key: value`, case-insensitive keys, `#`/`;` comment lines, `;` inline comments,
/// indented continuation lines and a `[DEFAULT]` section that every other section inherits.
#[derive(Debug, Default)]
struct IniDocument {
    defaults: Vec<(String, String)>,
    sections: Vec<(String, Vec<(String, String)>)>,
}

impl IniDocument {
    fn parse(text: &str, origin: &Path) -> Result<IniDocument> {
        let mut doc = IniDocument::default();
        // None = no section header seen yet; Some(None) = DEFAULT; Some(Some(i)) = sections[i].
        let mut current: Option<Option<usize>> = None;
        let mut last_key: Option<String> = None;
        for (line_no, raw) in text.lines().enumerate() {
            let line_no = line_no + 1;
            let stripped = raw.trim();
            if stripped.starts_with('#') || stripped.starts_with(';') {
                continue;
            }
            let value_part = strip_inline_comment(raw);
            if value_part.trim().is_empty() {
                last_key = None;
                continue;
            }
            let indented = raw.starts_with(char::is_whitespace);
            if indented {
                if let (Some(section), Some(key)) = (current, last_key.as_ref()) {
                    let entries = doc.entries_mut(section);
                    if let Some(entry) = entries.iter_mut().find(|(k, _)| k == key) {
                        entry.1.push('\n');
                        entry.1.push_str(value_part.trim());
                        continue;
                    }
                }
            }
            let line = value_part.trim();
            if line.starts_with('[') && line.ends_with(']') {
                let name = &line[1..line.len() - 1];
                last_key = None;
                if name == "DEFAULT" {
                    current = Some(None);
                } else if let Some(index) = doc.sections.iter().position(|(n, _)| n == name) {
                    current = Some(Some(index));
                } else {
                    doc.sections.push((name.to_owned(), Vec::new()));
                    current = Some(Some(doc.sections.len() - 1));
                }
                continue;
            }
            let Some(section) = current else {
                return Err(error::value_error(format!(
                    "File contains no section headers.\nfile: {}, line: {line_no}\n{}",
                    repr_str(&origin.display().to_string()),
                    repr_str(&format!("{raw}\n"))
                )));
            };
            let Some(split) = line.find(['=', ':']) else {
                return Err(error::value_error(format!(
                    "Source contains parsing errors: {}\n\t[line {line_no:2}]: {}",
                    repr_str(&origin.display().to_string()),
                    repr_str(&format!("{raw}\n"))
                )));
            };
            let key = line[..split].trim().to_lowercase();
            let value = line[split + 1..].trim().to_owned();
            let entries = doc.entries_mut(section);
            match entries.iter_mut().find(|(k, _)| *k == key) {
                Some(entry) => entry.1 = value,
                None => entries.push((key.clone(), value)),
            }
            last_key = Some(key);
        }
        Ok(doc)
    }

    fn entries_mut(&mut self, section: Option<usize>) -> &mut Vec<(String, String)> {
        match section {
            None => &mut self.defaults,
            Some(index) => &mut self.sections[index].1,
        }
    }
}

/// Inline comments start with `;` preceded by whitespace.
fn strip_inline_comment(line: &str) -> &str {
    let mut previous_is_space = false;
    for (index, c) in line.char_indices() {
        if c == ';' && previous_is_space {
            return &line[..index];
        }
        previous_is_space = c.is_whitespace();
    }
    line
}

pub struct EnterpriseConfiguration {
    doc: IniDocument,
}

impl Service for EnterpriseConfiguration {}

impl EnterpriseConfiguration {
    pub fn new() -> Result<Self> {
        Self::from_path(&default_path())
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read(path).map_err(|_| {
            error::file_not_found_error(format!("enterprise configuration missing: {}", path.display()))
        })?;
        let text = String::from_utf8(text)
            .map_err(|e| error::value_error(format!("'utf-8' codec can't decode configuration: {e}")))?;
        Ok(EnterpriseConfiguration { doc: IniDocument::parse(&text, path)? })
    }

    pub fn get(&self, section: &str, key: &str) -> Result<String> {
        let entries = &self
            .doc
            .sections
            .iter()
            .find(|(name, _)| name == section)
            .ok_or_else(|| error::no_section_error(format!("No section: {}", repr_str(section))))?
            .1;
        let key = key.to_lowercase();
        entries.iter().chain(self.doc.defaults.iter()).find(|(k, _)| *k == key).map(|(_, v)| v.clone()).ok_or_else(
            || error::no_option_error(format!("No option {} in section: {}", repr_str(&key), repr_str(section))),
        )
    }

    pub fn get_int(&self, section: &str, key: &str) -> Result<i64> {
        let raw = self.get(section, key)?;
        parse_int_literal(&raw)
            .ok_or_else(|| error::value_error(format!("invalid literal for int() with base 0: {}", repr_str(&raw))))
    }

    pub fn get_float(&self, section: &str, key: &str) -> Result<f64> {
        let raw = self.get(section, key)?;
        raw.trim()
            .replace('_', "")
            .parse()
            .map_err(|_| error::value_error(format!("could not convert string to float: {}", repr_str(&raw))))
    }

    pub fn get_bool(&self, section: &str, key: &str) -> Result<bool> {
        let raw = self.get(section, key)?;
        match raw.to_lowercase().as_str() {
            "1" | "yes" | "true" | "on" => Ok(true),
            "0" | "no" | "false" | "off" => Ok(false),
            _ => Err(error::value_error(format!("Not a boolean: {raw}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_comments_and_defaults() {
        let doc = IniDocument::parse(
            "; comment\n[DEFAULT]\nshared = 1\n[cpu]\nClock_Hz = 0x10 ; inline\nnote = a;b\nlong = one\n  two\n",
            Path::new("x.ini"),
        )
        .unwrap();
        let config = EnterpriseConfiguration { doc };
        assert_eq!(config.get_int("cpu", "clock_hz").unwrap(), 16);
        assert_eq!(config.get("cpu", "note").unwrap(), "a;b");
        assert_eq!(config.get("cpu", "long").unwrap(), "one\ntwo");
        assert_eq!(config.get("cpu", "shared").unwrap(), "1");
        assert_eq!(config.get("gpu", "x").unwrap_err().message, "No section: 'gpu'");
        assert_eq!(config.get("cpu", "x").unwrap_err().message, "No option 'x' in section: 'cpu'");
    }
}
