// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! File integrity verification.
//!
//! Every source file, firmware image, configuration file and main.rs is pinned
//! by SHA-256 in checksums.sha256. Modified, missing or unexpected files abort
//! startup before a single module is resolved.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::core::error::{self, Result};

const LOG: &str = "core.security.integrity";

pub fn checksums_path(root: &Path) -> PathBuf {
    root.join("src").join("core").join("security").join("checksums.sha256")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| error::os_error(format!("{e}: {}", dir.display())))?;
    for entry in entries {
        let path = entry.map_err(|e| error::os_error(e.to_string()))?.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

pub fn protected_files(root: &Path) -> Result<Vec<PathBuf>> {
    let src = root.join("src");
    let mut files = vec![src.join("main.rs"), src.join("lib.rs")];
    let mut package = Vec::new();
    walk(&src.join("core"), &mut package)?;
    package.sort();
    let checksums = checksums_path(root);
    files.extend(package.into_iter().filter(|path| *path != checksums));
    Ok(files)
}

pub fn sha256_of(path: &Path) -> Result<String> {
    let mut handle = std::fs::File::open(path).map_err(|e| error::os_error(format!("{e}: {}", path.display())))?;
    let mut digest = Sha256::new();
    let mut chunk = vec![0u8; 1 << 16];
    loop {
        let read = handle.read(&mut chunk).map_err(|e| error::os_error(e.to_string()))?;
        if read == 0 {
            break;
        }
        digest.update(&chunk[..read]);
    }
    Ok(digest.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn posix_relative(path: &Path, root: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

pub fn compute_manifest(root: &Path) -> Result<BTreeMap<String, String>> {
    protected_files(root)?.iter().map(|path| Ok((posix_relative(path, root), sha256_of(path)?))).collect()
}

pub fn load_manifest(path: &Path) -> Result<BTreeMap<String, String>> {
    if !path.exists() {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        return Err(error::integrity_violation(format!("integrity manifest missing: {name}")));
    }
    let text = std::fs::read_to_string(path).map_err(|e| error::os_error(e.to_string()))?;
    let mut entries = BTreeMap::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.trim_start();
        let Some((digest, name)) = line.split_once(char::is_whitespace) else {
            return Err(error::value_error("not enough values to unpack (expected 2, got 1)"));
        };
        entries.insert(name.trim().to_owned(), digest.to_owned());
    }
    Ok(entries)
}

/// Renders a manifest in the checksums.sha256 format.
pub fn format_manifest(manifest: &BTreeMap<String, String>) -> String {
    manifest.iter().map(|(name, digest)| format!("{digest}  {name}\n")).collect()
}

pub fn verify() -> Result<usize> {
    verify_at(&crate::core::project_root())
}

pub fn verify_at(root: &Path) -> Result<usize> {
    let expected = load_manifest(&checksums_path(root))?;
    let actual = compute_manifest(root)?;
    let missing: Vec<&str> = expected.keys().filter(|k| !actual.contains_key(*k)).map(String::as_str).collect();
    let unexpected: Vec<&str> = actual.keys().filter(|k| !expected.contains_key(*k)).map(String::as_str).collect();
    let modified: Vec<&str> = expected
        .iter()
        .filter(|(name, digest)| actual.get(*name).is_some_and(|d| d != *digest))
        .map(|(name, _)| name.as_str())
        .collect();
    if !(missing.is_empty() && unexpected.is_empty() && modified.is_empty()) {
        let details: Vec<String> = [("modified", modified), ("missing", missing), ("unexpected", unexpected)]
            .into_iter()
            .filter(|(_, names)| !names.is_empty())
            .map(|(label, names)| format!("{label}: {}", names.join(", ")))
            .collect();
        return Err(error::integrity_violation(format!("file integrity check failed ({})", details.join("; "))));
    }
    crate::log_info!(LOG, "integrity: {} files verified against SHA-256 manifest", actual.len());
    Ok(actual.len())
}
