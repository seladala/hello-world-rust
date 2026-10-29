//! A logging facility in the image of the standard library of a slower language.
//!
//! Records go to stderr as `[LEVEL] logger.name: message`, with the level name
//! left-aligned in five columns, exactly as `%(levelname)-5s` would do it.

use std::io::Write;
use std::sync::atomic::{AtomicU8, Ordering};

pub const DEBUG: u8 = 10;
pub const INFO: u8 = 20;
pub const WARNING: u8 = 30;
pub const ERROR: u8 = 40;

static THRESHOLD: AtomicU8 = AtomicU8::new(WARNING);

pub fn basic_config(level: u8) {
    THRESHOLD.store(level, Ordering::Relaxed);
}

pub fn enabled(level: u8) -> bool {
    level >= THRESHOLD.load(Ordering::Relaxed)
}

pub fn level_name(level: u8) -> &'static str {
    match level {
        DEBUG => "DEBUG",
        INFO => "INFO",
        WARNING => "WARNING",
        ERROR => "ERROR",
        _ => "Level",
    }
}

pub fn emit(level: u8, logger: &str, message: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "[{:<5}] {}: {}", level_name(level), logger, message);
}

#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $($arg:tt)*) => {
        if $crate::core::logging::enabled($crate::core::logging::DEBUG) {
            $crate::core::logging::emit($crate::core::logging::DEBUG, $logger, &format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $($arg:tt)*) => {
        if $crate::core::logging::enabled($crate::core::logging::INFO) {
            $crate::core::logging::emit($crate::core::logging::INFO, $logger, &format!($($arg)*));
        }
    };
}
