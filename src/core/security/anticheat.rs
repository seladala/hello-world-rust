//! Runtime anti-tamper and anti-cheat protection.
//!
//! Detects debuggers, profilers, interactive inspection, patching of critical
//! code paths, output stream hijacking and clock manipulation ("speedhacks").
//! Scans run at boot and again right before rendering.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

use sha2::{Digest, Sha256};

use crate::core::config::EnterpriseConfiguration;
use crate::core::container::Service;
use crate::core::error::{self, Result};
use crate::core::events::EventBus;
use crate::core::output::terminal::TerminalRenderingEngine;
use crate::core::runtime::executor::Machine;
use crate::core::validation::normalization::LetterCaseNegotiator;
use crate::core::validation::policy::GlyphApprovalCommittee;
use crate::core::values::Value;

const LOG: &str = "core.security.anticheat";

/// How many bytes of machine code are fingerprinted per protected method.
const CODE_WINDOW: usize = 32;

type ProtectedClass = (&'static str, Vec<(&'static str, *const ())>);

fn protected_classes() -> Vec<ProtectedClass> {
    vec![
        (
            "core.runtime.executor.Machine",
            vec![
                ("load", Machine::load as *const ()),
                ("run", Machine::run as *const ()),
                ("step", Machine::step as *const ()),
                ("_op_add", Machine::op_add as *const ()),
                ("_op_and", Machine::op_and as *const ()),
                ("_op_call", Machine::op_call as *const ()),
                ("_op_cmpi", Machine::op_cmpi as *const ()),
                ("_op_in", Machine::op_in as *const ()),
                ("_op_ldb", Machine::op_ldb as *const ()),
                ("_op_out", Machine::op_out as *const ()),
                ("_op_ret", Machine::op_ret as *const ()),
                ("_op_xor", Machine::op_xor as *const ()),
            ],
        ),
        (
            "core.validation.policy.GlyphApprovalCommittee",
            vec![("review", GlyphApprovalCommittee::review as *const ())],
        ),
        (
            "core.validation.normalization.LetterCaseNegotiator",
            vec![
                ("negotiate", LetterCaseNegotiator::negotiate as *const ()),
                ("_enforce_separators", LetterCaseNegotiator::enforce_separators as *const ()),
            ],
        ),
        (
            "core.output.terminal.TerminalRenderingEngine",
            vec![
                ("_on_glyph", TerminalRenderingEngine::on_glyph as *const ()),
                ("_on_complete", TerminalRenderingEngine::on_complete as *const ()),
            ],
        ),
    ]
}

/// The first bytes of a function's machine code, or nothing where code is not readable data.
fn code_bytes(entry: *const ()) -> &'static [u8] {
    if cfg!(all(any(target_arch = "x86_64", target_arch = "aarch64"), any(target_os = "linux", target_os = "macos"))) {
        // SAFETY: on these targets functions live in mapped, readable text pages, and a short
        // window past any function entry stays inside the executable's text segment.
        unsafe { std::slice::from_raw_parts(entry as *const u8, CODE_WINDOW) }
    } else {
        &[]
    }
}

fn code_fingerprint(methods: &[(&'static str, *const ())]) -> String {
    let mut sorted = methods.to_vec();
    sorted.sort_by_key(|(name, _)| *name);
    let mut digest = Sha256::new();
    for (name, entry) in sorted {
        digest.update(name.as_bytes());
        digest.update(code_bytes(entry));
    }
    digest.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn snapshot() -> BTreeMap<String, String> {
    protected_classes().into_iter().map(|(name, methods)| (name.to_owned(), code_fingerprint(&methods))).collect()
}

/// Ratio of wall-clock to monotonic time over a short busy wait.
pub fn clock_ratio(duration: Duration) -> f64 {
    let (wall_start, mono_start) = (SystemTime::now(), Instant::now());
    while mono_start.elapsed() < duration {}
    let mono = mono_start.elapsed().as_secs_f64();
    let wall = SystemTime::now().duration_since(wall_start).map(|d| d.as_secs_f64()).unwrap_or(-1.0);
    wall / mono
}

/// Identity of whatever standard output currently points at.
#[cfg(unix)]
fn stdout_identity() -> Option<(u64, u64)> {
    use std::os::fd::AsFd;
    use std::os::unix::fs::MetadataExt;
    let fd = std::io::stdout().as_fd().try_clone_to_owned().ok()?;
    let metadata = std::fs::File::from(fd).metadata().ok()?;
    Some((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn stdout_identity() -> Option<(u64, u64)> {
    None
}

fn tracer_attached() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| status.lines().find_map(|line| line.strip_prefix("TracerPid:").map(|pid| pid.trim() != "0")))
        .unwrap_or(false)
}

fn env_flag(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

pub struct AntiCheatEngine {
    tolerance: f64,
    bus: Rc<EventBus>,
    baseline: RefCell<BTreeMap<String, String>>,
    stdout: Cell<Option<(u64, u64)>>,
    pub scans: Cell<u64>,
}

impl AntiCheatEngine {
    pub fn new(config: &EnterpriseConfiguration, event_bus: Rc<EventBus>) -> Result<Self> {
        Ok(AntiCheatEngine {
            tolerance: config.get_float("security", "clock_tolerance")?,
            bus: event_bus,
            baseline: RefCell::new(BTreeMap::new()),
            stdout: Cell::new(None),
            scans: Cell::new(0),
        })
    }

    pub fn scan(&self) -> Result<()> {
        if tracer_attached() {
            return Err(error::cheat_detected("debugger or tracer attached"));
        }
        if env_flag("LD_PRELOAD") || env_flag("DYLD_INSERT_LIBRARIES") {
            return Err(error::cheat_detected("profiler attached"));
        }
        if env_flag("PYTHONINSPECT") {
            return Err(error::cheat_detected("interactive inspection mode enabled"));
        }
        if stdout_identity() != self.stdout.get() {
            return Err(error::cheat_detected("standard output was hijacked after boot"));
        }
        let baseline = self.baseline.borrow();
        let tampered: Vec<String> =
            snapshot().into_iter().filter(|(name, fp)| baseline.get(name) != Some(fp)).map(|(name, _)| name).collect();
        if !tampered.is_empty() {
            return Err(error::cheat_detected(format!(
                "runtime code modification detected in {}",
                tampered.join(", ")
            )));
        }
        let ratio = clock_ratio(Duration::from_millis(5));
        if (ratio - 1.0).abs() > self.tolerance {
            return Err(error::cheat_detected(format!("clock manipulation detected (ratio {ratio:.2})")));
        }
        self.scans.set(self.scans.get() + 1);
        self.bus.publish("anticheat.scan.clean", Value::Int(self.scans.get() as i64))?;
        crate::log_info!(LOG, "anti-cheat: scan #{} clean ({} protected classes)", self.scans.get(), baseline.len());
        Ok(())
    }

    /// Detect self-modifying firmware or memory editing during execution.
    pub fn verify_code_segment(&self, machine: &Machine, image: &[u8]) -> Result<()> {
        if machine.memory.get(..image.len()) != Some(image) {
            return Err(error::cheat_detected("firmware code segment was modified during execution"));
        }
        Ok(())
    }
}

impl Service for AntiCheatEngine {
    fn on_start(&self) -> Result<()> {
        *self.baseline.borrow_mut() = snapshot();
        self.stdout.set(stdout_identity());
        self.scan()
    }
}
