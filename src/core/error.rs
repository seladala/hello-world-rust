// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
//! Exception hierarchy. Every way the program may refuse to greet you, in one enum.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    // SecurityViolation and its subclasses.
    IntegrityViolation,
    CheatDetected,
    LicenseError,
    ComplianceViolation,
    // Everything else.
    AssemblyError,
    AttributeError,
    ContainerError,
    DependencyError,
    FileNotFoundError,
    GlyphRejected,
    IndexError,
    KeyError,
    LedgerCorruption,
    MachineFault,
    ModuleNotFoundError,
    NegotiationDeadlock,
    NoOptionError,
    NoSectionError,
    OsError,
    PiConsensusFailure,
    RuntimeError,
    TypeError,
    ValueError,
    WarmupFailure,
}

impl ErrorKind {
    /// Fully qualified exception name, as the interpreter of the original would print it.
    pub fn qualified_name(self) -> &'static str {
        match self {
            ErrorKind::IntegrityViolation => "core.security.errors.IntegrityViolation",
            ErrorKind::CheatDetected => "core.security.errors.CheatDetected",
            ErrorKind::LicenseError => "core.security.errors.LicenseError",
            ErrorKind::ComplianceViolation => "__main__.ComplianceViolation",
            ErrorKind::AssemblyError => "core.runtime.compiler.AssemblyError",
            ErrorKind::AttributeError => "AttributeError",
            ErrorKind::ContainerError => "core.container.ContainerError",
            ErrorKind::DependencyError => "core.modules.graph.DependencyError",
            ErrorKind::FileNotFoundError => "FileNotFoundError",
            ErrorKind::GlyphRejected => "core.validation.policy.GlyphRejected",
            ErrorKind::IndexError => "IndexError",
            ErrorKind::KeyError => "KeyError",
            ErrorKind::LedgerCorruption => "core.validation.audit_log.LedgerCorruption",
            ErrorKind::MachineFault => "core.runtime.executor.MachineFault",
            ErrorKind::ModuleNotFoundError => "ModuleNotFoundError",
            ErrorKind::NegotiationDeadlock => "core.validation.normalization.NegotiationDeadlock",
            ErrorKind::NoOptionError => "configparser.NoOptionError",
            ErrorKind::NoSectionError => "configparser.NoSectionError",
            ErrorKind::OsError => "OSError",
            ErrorKind::PiConsensusFailure => "core.numerics.quorum.PiConsensusFailure",
            ErrorKind::RuntimeError => "RuntimeError",
            ErrorKind::TypeError => "TypeError",
            ErrorKind::ValueError => "ValueError",
            ErrorKind::WarmupFailure => "core.diagnostics.health_check.WarmupFailure",
        }
    }

    /// Base class for every reason the program may refuse to greet you.
    pub fn is_security_violation(self) -> bool {
        matches!(
            self,
            ErrorKind::IntegrityViolation
                | ErrorKind::CheatDetected
                | ErrorKind::LicenseError
                | ErrorKind::ComplianceViolation
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreError {
    pub kind: ErrorKind,
    pub message: String,
}

impl CoreError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        CoreError { kind, message: message.into() }
    }

    pub fn is_security_violation(&self) -> bool {
        self.kind.is_security_violation()
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CoreError {}

pub type Result<T> = std::result::Result<T, CoreError>;

macro_rules! constructors {
    ($($fn_name:ident => $kind:ident),* $(,)?) => {
        $(
            pub fn $fn_name(message: impl Into<String>) -> CoreError {
                CoreError::new(ErrorKind::$kind, message)
            }
        )*
    };
}

constructors! {
    integrity_violation => IntegrityViolation,
    cheat_detected => CheatDetected,
    license_error => LicenseError,
    compliance_violation => ComplianceViolation,
    assembly_error => AssemblyError,
    attribute_error => AttributeError,
    container_error => ContainerError,
    dependency_error => DependencyError,
    file_not_found_error => FileNotFoundError,
    glyph_rejected => GlyphRejected,
    index_error => IndexError,
    key_error => KeyError,
    ledger_corruption => LedgerCorruption,
    machine_fault => MachineFault,
    module_not_found_error => ModuleNotFoundError,
    negotiation_deadlock => NegotiationDeadlock,
    no_option_error => NoOptionError,
    no_section_error => NoSectionError,
    os_error => OsError,
    pi_consensus_failure => PiConsensusFailure,
    runtime_error => RuntimeError,
    type_error => TypeError,
    value_error => ValueError,
    warmup_failure => WarmupFailure,
}
