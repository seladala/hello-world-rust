//! Hello World core library.

use std::path::PathBuf;

pub mod argparse;
pub mod bootstrap;
pub mod config;
pub mod container;
pub mod diagnostics;
pub mod error;
pub mod events;
pub mod json;
pub mod logging;
pub mod modules;
pub mod numerics;
pub mod output;
pub mod runtime;
pub mod security;
pub mod services;
pub mod validation;
pub mod values;

pub const VERSION: &str = "6.7.0";

/// Environment variable that relocates the project root (the directory holding `src/`).
pub const ROOT_ENV: &str = "HELLO_WORLD_ROOT";

/// The project root: where `src/main.rs`, `src/lib.rs` and `src/core/` live at runtime.
pub fn project_root() -> PathBuf {
    match std::env::var_os(ROOT_ENV) {
        Some(root) if !root.is_empty() => PathBuf::from(root),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    }
}

/// The `core` package directory, home of every configuration file and firmware image.
pub fn package_dir() -> PathBuf {
    project_root().join("src").join("core")
}
