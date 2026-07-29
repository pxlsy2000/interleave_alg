//! Typed boundary and domain failures with exact process-exit routing.

use thiserror::Error;

use crate::issue::IssueCode;

/// Fixed process-exit classes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExitClass {
    /// Successful completion, including warnings.
    Success,
    /// Usage, input/output access, or output transaction failure.
    UsageOrIo,
    /// Mapping input, support, or validation failure.
    Mapping,
    /// Scenario, selection, resource, or address failure.
    ScenarioOrAddress,
    /// Unexpected failure after complete below-limit preflight.
    Analysis,
}

impl ExitClass {
    /// Returns the exact process exit code.
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::UsageOrIo => 1,
            Self::Mapping => 2,
            Self::ScenarioOrAddress => 3,
            Self::Analysis => 4,
        }
    }
}

/// The schema owning an input boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputKind {
    /// Mapping source.
    Mapping,
    /// Scenario source.
    Scenario,
}

/// Command-line, input, and output boundary failures.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum BoundaryError {
    /// Command-line usage is invalid.
    #[error("invalid command-line usage")]
    Usage,
    /// An input source could not be read.
    #[error("input could not be read")]
    InputIo {
        /// Schema owning the source.
        kind: InputKind,
    },
    /// Existing output was refused without force.
    #[error("output path already exists; use --force to replace it")]
    OutputExists,
    /// Existing output is not an acceptable regular file.
    #[error("output target must be a regular file")]
    OutputInvalidTarget,
    /// Atomic no-clobber commit is unavailable.
    #[error("atomic no-clobber rename is unsupported")]
    OutputAtomicUnsupported,
    /// Output writing or commit failed.
    #[error("report output could not be completed")]
    OutputIo,
    /// Complete rendered report exceeds the v1 bound.
    #[error("report exceeds v1 limit 268435456 bytes")]
    OutputTooLarge,
}

impl BoundaryError {
    /// Returns the boundary exit class.
    pub const fn exit_class(self) -> ExitClass {
        match self {
            Self::Usage
            | Self::InputIo { .. }
            | Self::OutputExists
            | Self::OutputInvalidTarget
            | Self::OutputAtomicUnsupported
            | Self::OutputIo
            | Self::OutputTooLarge => ExitClass::UsageOrIo,
        }
    }

    /// Returns the stable issue code when the failure has one.
    pub const fn issue_code(self) -> Option<IssueCode> {
        match self {
            Self::Usage => None,
            Self::InputIo { .. } => Some(IssueCode::InputIo),
            Self::OutputExists => Some(IssueCode::OutputExists),
            Self::OutputInvalidTarget => Some(IssueCode::OutputInvalidTarget),
            Self::OutputAtomicUnsupported => Some(IssueCode::OutputAtomicUnsupported),
            Self::OutputIo => Some(IssueCode::OutputIo),
            Self::OutputTooLarge => Some(IssueCode::OutputTooLarge),
        }
    }
}

/// Typed failures after the process boundary has been acquired.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum DomainError {
    /// Input failed schema-specific parsing or validation.
    #[error("input is invalid")]
    InvalidInput {
        /// Schema owning the invalid source.
        kind: InputKind,
    },
    /// Mapping is meaningful but outside v1.
    #[error("mapping is unsupported")]
    UnsupportedMapping,
    /// Mapping failed dimensions or mathematics.
    #[error("mapping is invalid")]
    InvalidMapping,
    /// Scenario failed selection, semantics, or resource preflight.
    #[error("scenario is invalid")]
    InvalidScenario,
    /// Query or generated address is invalid or out of range.
    #[error("address is invalid")]
    InvalidAddress,
    /// Valid below-limit analysis failed unexpectedly.
    #[error("analysis could not be completed")]
    AnalysisFailed,
}

impl DomainError {
    /// Returns the schema-specific process exit class.
    pub const fn exit_class(self) -> ExitClass {
        match self {
            Self::InvalidInput {
                kind: InputKind::Mapping,
            }
            | Self::UnsupportedMapping
            | Self::InvalidMapping => ExitClass::Mapping,
            Self::InvalidInput {
                kind: InputKind::Scenario,
            }
            | Self::InvalidScenario
            | Self::InvalidAddress => ExitClass::ScenarioOrAddress,
            Self::AnalysisFailed => ExitClass::Analysis,
        }
    }
}

/// A process-level typed failure.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ApplicationError {
    /// Boundary failure.
    #[error(transparent)]
    Boundary(#[from] BoundaryError),
    /// Domain failure.
    #[error(transparent)]
    Domain(#[from] DomainError),
}

impl ApplicationError {
    /// Returns the process exit class.
    pub const fn exit_class(self) -> ExitClass {
        match self {
            Self::Boundary(error) => error.exit_class(),
            Self::Domain(error) => error.exit_class(),
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
