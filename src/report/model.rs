use serde::Serialize;
use thiserror::Error;

use crate::{
    issue::{Issue, ReportStatus, Severity, report_status, sort_issues},
    mapping::{MappedAddress, MappingModel, MappingValidation},
};

use super::{
    MapResult, RunCaseResult, RunResult, ValidateResult, model_mapping::valid_query_classification,
};

/// Command whose outcome is represented by a report.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportCommand {
    /// Mapping validation.
    Validate,
    /// Address Mapping query.
    Map,
    /// Scenario analysis.
    Run,
}

/// Stable public issue representation without internal ordering metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReportIssue {
    pub(super) code: &'static str,
    pub(super) path: String,
    pub(super) message: String,
}

impl From<&Issue> for ReportIssue {
    fn from(issue: &Issue) -> Self {
        Self {
            code: issue.code().as_str(),
            path: issue.path().as_str().to_owned(),
            message: issue.message().to_owned(),
        }
    }
}

/// Typed result objects for all report commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ReportResult {
    /// Mapping validation result.
    Validate(ValidateResult),
    /// Address Mapping query result.
    Map(MapResult),
    /// Scenario analysis result.
    Run(RunResult),
}

/// One complete, fixed-key JSON report envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub(super) schema_version: u8,
    pub(super) command: ReportCommand,
    pub(super) status: ReportStatus,
    pub(super) warnings: Vec<ReportIssue>,
    pub(super) errors: Vec<ReportIssue>,
    pub(super) result: Option<ReportResult>,
}

impl Report {
    /// Constructs a structural, preflight, or analysis failure with no partial result.
    ///
    /// # Errors
    /// Returns an error if the issue list contains no error.
    pub fn failure(
        command: ReportCommand,
        mut issues: Vec<Issue>,
    ) -> Result<Self, ReportModelError> {
        sort_issues(&mut issues);
        let status = report_status(&issues);
        if status != ReportStatus::Fail {
            return Err(ReportModelError::FailureWithoutError);
        }
        let (warnings, errors) = split_issues(&issues);
        Ok(Self::new(command, status, warnings, errors, None))
    }

    /// Constructs a completed Mapping validation report.
    pub fn validate(mapping: &MappingModel, input: &str, validation: &MappingValidation) -> Self {
        let issues = validation.issues();
        let (warnings, errors) = split_issues(issues);
        Self::new(
            ReportCommand::Validate,
            report_status(issues),
            warnings,
            errors,
            Some(ReportResult::Validate(ValidateResult::new(
                mapping, input, validation,
            ))),
        )
    }

    /// Constructs a completed address Mapping report.
    ///
    /// # Errors
    /// Returns an error for an invalid Mapping classification or an error issue.
    pub fn map(
        mapping: &MappingModel,
        validation: &MappingValidation,
        addresses: &[MappedAddress],
    ) -> Result<Self, ReportModelError> {
        Self::map_with_input(mapping, None, validation, addresses)
    }

    /// Constructs a completed address Mapping report with text presentation context.
    pub fn map_with_input(
        mapping: &MappingModel,
        input: Option<&str>,
        validation: &MappingValidation,
        addresses: &[MappedAddress],
    ) -> Result<Self, ReportModelError> {
        valid_query_classification(validation.classification())?;
        report_with_result(
            ReportCommand::Map,
            validation.issues(),
            ReportResult::Map(MapResult::new(mapping, validation, addresses, input)),
        )
    }

    /// Constructs a completed Scenario analysis report.
    ///
    /// # Errors
    /// Returns an error for an invalid Mapping classification, an error issue,
    /// or a report value outside the JSON-safe integer contract.
    pub fn run(
        mapping: &MappingModel,
        validation: &MappingValidation,
        cases: Vec<RunCaseResult>,
    ) -> Result<Self, ReportModelError> {
        Self::run_with_input(mapping, None, validation, cases)
    }

    /// Constructs a completed Scenario report with text presentation context.
    pub fn run_with_input(
        mapping: &MappingModel,
        input: Option<&str>,
        validation: &MappingValidation,
        cases: Vec<RunCaseResult>,
    ) -> Result<Self, ReportModelError> {
        valid_query_classification(validation.classification())?;
        report_with_result(
            ReportCommand::Run,
            validation.issues(),
            ReportResult::Run(RunResult::new(mapping, validation, cases, input)),
        )
    }

    const fn new(
        command: ReportCommand,
        status: ReportStatus,
        warnings: Vec<ReportIssue>,
        errors: Vec<ReportIssue>,
        result: Option<ReportResult>,
    ) -> Self {
        Self {
            schema_version: 1,
            command,
            status,
            warnings,
            errors,
            result,
        }
    }
}

fn report_with_result(
    command: ReportCommand,
    issues: &[Issue],
    result: ReportResult,
) -> Result<Report, ReportModelError> {
    let status = report_status(issues);
    if status == ReportStatus::Fail {
        return Err(ReportModelError::SuccessfulResultWithError);
    }
    let (warnings, errors) = split_issues(issues);
    Ok(Report::new(command, status, warnings, errors, Some(result)))
}

fn split_issues(issues: &[Issue]) -> (Vec<ReportIssue>, Vec<ReportIssue>) {
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity() == Severity::Warning)
        .map(ReportIssue::from)
        .collect();
    let errors = issues
        .iter()
        .filter(|issue| issue.severity() == Severity::Error)
        .map(ReportIssue::from)
        .collect();
    (warnings, errors)
}

/// Invalid assembly of an otherwise typed report.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ReportModelError {
    /// A failure envelope was requested without an error issue.
    #[error("failure report requires at least one error")]
    FailureWithoutError,
    /// A successful result was paired with an error issue.
    #[error("completed result cannot contain an error issue")]
    SuccessfulResultWithError,
    /// Map or run was requested for an invalid Mapping.
    #[error("map and run results require a valid Mapping")]
    InvalidMappingClassification,
    /// A report integer exceeded the JavaScript-safe range.
    #[error("report integer exceeds the JSON safe-integer contract")]
    UnsafeInteger,
    /// Exact six-place ratio formatting failed.
    #[error("ratio cannot be represented by the JSON contract")]
    InvalidRatio,
}
