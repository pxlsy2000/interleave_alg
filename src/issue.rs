//! Stable diagnostics, path encoding, ordering, and report statuses.

mod escape;

pub use escape::canonical_json_string;

/// Stable external issue identifiers.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IssueCode {
    /// Input source could not be read.
    InputIo,
    /// YAML syntax, encoding, document, or prohibited-feature failure.
    InputYamlParse,
    /// Unknown schema field.
    InputUnknownField,
    /// Invalid input value or resource count.
    InputInvalidValue,
    /// Meaningful Mapping outside the v1 support envelope.
    MappingUnsupported,
    /// Target matrix is not full row rank.
    MappingTargetUnreachable,
    /// Combined Mapping is not bijective.
    MappingNonBijective,
    /// Bijective Mapping with non-natural local-address order.
    MappingNonNatural,
    /// Invalid Scenario or expanded-run input.
    ScenarioInvalid,
    /// Requested Scenario case does not exist.
    ScenarioCaseNotFound,
    /// No Scenario case was selected.
    ScenarioNoCaseSelected,
    /// Invalid address lexeme.
    AddressInvalid,
    /// Address outside the configured width.
    AddressOutOfRange,
    /// Existing output refused without force.
    OutputExists,
    /// Output target is not an acceptable regular file.
    OutputInvalidTarget,
    /// Atomic no-clobber replacement is unavailable.
    OutputAtomicUnsupported,
    /// Output I/O failed.
    OutputIo,
    /// Rendered report exceeds the byte cap.
    OutputTooLarge,
    /// Valid below-limit analysis failed unexpectedly.
    AnalysisFailed,
}

impl IssueCode {
    /// Returns the stable dotted external code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InputIo => "input.io",
            Self::InputYamlParse => "input.yaml_parse",
            Self::InputUnknownField => "input.unknown_field",
            Self::InputInvalidValue => "input.invalid_value",
            Self::MappingUnsupported => "mapping.unsupported",
            Self::MappingTargetUnreachable => "mapping.target_unreachable",
            Self::MappingNonBijective => "mapping.non_bijective",
            Self::MappingNonNatural => "mapping.non_natural",
            Self::ScenarioInvalid => "scenario.invalid",
            Self::ScenarioCaseNotFound => "scenario.case_not_found",
            Self::ScenarioNoCaseSelected => "scenario.no_case_selected",
            Self::AddressInvalid => "address.invalid",
            Self::AddressOutOfRange => "address.out_of_range",
            Self::OutputExists => "output.exists",
            Self::OutputInvalidTarget => "output.invalid_target",
            Self::OutputAtomicUnsupported => "output.atomic_unsupported",
            Self::OutputIo => "output.io",
            Self::OutputTooLarge => "output.too_large",
            Self::AnalysisFailed => "analysis.failed",
        }
    }

    /// Returns the fixed severity associated with the code.
    pub const fn severity(self) -> Severity {
        match self {
            Self::MappingNonNatural => Severity::Warning,
            Self::InputIo
            | Self::InputYamlParse
            | Self::InputUnknownField
            | Self::InputInvalidValue
            | Self::MappingUnsupported
            | Self::MappingTargetUnreachable
            | Self::MappingNonBijective
            | Self::ScenarioInvalid
            | Self::ScenarioCaseNotFound
            | Self::ScenarioNoCaseSelected
            | Self::AddressInvalid
            | Self::AddressOutOfRange
            | Self::OutputExists
            | Self::OutputInvalidTarget
            | Self::OutputAtomicUnsupported
            | Self::OutputIo
            | Self::OutputTooLarge
            | Self::AnalysisFailed => Severity::Error,
        }
    }

    const fn phase(self) -> IssuePhase {
        match self {
            Self::InputIo => IssuePhase::InputRead,
            Self::InputYamlParse => IssuePhase::Yaml,
            Self::InputUnknownField | Self::InputInvalidValue => IssuePhase::Schema,
            Self::MappingUnsupported => IssuePhase::MappingScalar,
            Self::MappingTargetUnreachable
            | Self::MappingNonBijective
            | Self::MappingNonNatural => IssuePhase::MappingMath,
            Self::ScenarioCaseNotFound | Self::ScenarioNoCaseSelected => {
                IssuePhase::ScenarioSelection
            }
            Self::ScenarioInvalid => IssuePhase::ScenarioSemantic,
            Self::AddressInvalid | Self::AddressOutOfRange => IssuePhase::Address,
            Self::AnalysisFailed => IssuePhase::Analysis,
            Self::OutputExists
            | Self::OutputInvalidTarget
            | Self::OutputAtomicUnsupported
            | Self::OutputIo
            | Self::OutputTooLarge => IssuePhase::Output,
        }
    }
}

/// Fixed issue severity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    /// Non-fatal diagnostic.
    Warning,
    /// Fatal diagnostic.
    Error,
}

/// Top-level report status.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    /// No issues.
    Pass,
    /// At least one warning and no error.
    Warning,
    /// At least one error.
    Fail,
}

impl ReportStatus {
    /// Returns the fixed lowercase envelope value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warning => "warning",
            Self::Fail => "fail",
        }
    }
}

/// Per-check status.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// Check passed.
    Pass,
    /// Check failed without invalidating the Mapping.
    Warning,
    /// Check failed and invalidated the Mapping.
    Fail,
}

impl CheckStatus {
    /// Returns the fixed lowercase report value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warning => "warning",
            Self::Fail => "fail",
        }
    }
}

/// Validation phases in their fixed diagnostic order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IssuePhase {
    /// Bounded input acquisition.
    InputRead,
    /// Source encoding and YAML syntax.
    Yaml,
    /// Whole-document schema decoding.
    Schema,
    /// Mapping scalar, relation, and support-cap validation.
    MappingScalar,
    /// Mapping dimensions and taps.
    MappingShape,
    /// Mapping mathematical checks.
    MappingMath,
    /// Scenario case selection.
    ScenarioSelection,
    /// Selected Scenario semantics.
    ScenarioSemantic,
    /// Expansion and resource preflight.
    Expansion,
    /// Query or generated-address validation.
    Address,
    /// Below-limit analysis.
    Analysis,
    /// Report rendering and output.
    Output,
}

/// A total deterministic issue-order key.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IssueOrderKey {
    phase: IssuePhase,
    source_order: usize,
    field_order: usize,
}

impl IssueOrderKey {
    /// Constructs a phase/source/field ordering key.
    pub const fn new(phase: IssuePhase, source_order: usize, field_order: usize) -> Self {
        Self {
            phase,
            source_order,
            field_order,
        }
    }
}

/// A stable diagnostic with typed code, path, message, and ordering metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Issue {
    code: IssueCode,
    path: IssuePath,
    message: String,
    order: IssueOrderKey,
}

impl Issue {
    /// Constructs an issue at the code's default validation phase.
    pub fn new(code: IssueCode, path: IssuePath, message: impl Into<String>) -> Self {
        Self {
            code,
            path,
            message: message.into(),
            order: IssueOrderKey::new(code.phase(), 0, 0),
        }
    }

    /// Replaces the issue's ordering metadata.
    #[must_use]
    pub const fn with_order(mut self, order: IssueOrderKey) -> Self {
        self.order = order;
        self
    }

    /// Returns the issue code.
    pub const fn code(&self) -> IssueCode {
        self.code
    }

    /// Returns the issue path.
    pub const fn path(&self) -> &IssuePath {
        &self.path
    }

    /// Returns the stable message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the fixed severity.
    pub const fn severity(&self) -> Severity {
        self.code.severity()
    }
}

/// A zero-based field path.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IssuePath(String);

impl IssuePath {
    /// Constructs the empty command-level path.
    pub const fn root() -> Self {
        Self(String::new())
    }

    /// Appends a known schema field.
    #[must_use]
    pub fn field(mut self, field: &str) -> Self {
        if !self.0.is_empty() {
            self.0.push('.');
        }
        self.0.push_str(field);
        self
    }

    /// Appends a zero-based sequence position.
    #[must_use]
    pub fn index(mut self, index: usize) -> Self {
        self.0.push('[');
        self.0.push_str(&index.to_string());
        self.0.push(']');
        self
    }

    /// Appends a raw user key as a canonical JSON bracket segment.
    #[must_use]
    pub fn raw_key(mut self, key: &str) -> Self {
        self.0.push('[');
        self.0.push_str(&canonical_json_string(key));
        self.0.push(']');
        self
    }

    /// Returns the rendered path.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Sorts issues by the fixed validation/source/field order.
pub fn sort_issues(issues: &mut [Issue]) {
    issues.sort_by_key(|issue| issue.order);
}

/// Derives the report status from issue severities.
pub fn report_status(issues: &[Issue]) -> ReportStatus {
    if issues
        .iter()
        .any(|issue| issue.severity() == Severity::Error)
    {
        ReportStatus::Fail
    } else if issues
        .iter()
        .any(|issue| issue.severity() == Severity::Warning)
    {
        ReportStatus::Warning
    } else {
        ReportStatus::Pass
    }
}

#[cfg(test)]
#[path = "issue_tests.rs"]
mod tests;
