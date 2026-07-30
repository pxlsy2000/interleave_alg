mod bounded;
mod json;
mod model;
mod model_mapping;
mod model_run;
mod text;

pub use bounded::OutputTooLarge;
pub use json::{JsonRenderError, render_json};
pub use model::{Report, ReportCommand, ReportIssue, ReportModelError, ReportResult};
pub use model_mapping::{DerivedMapping, MapAddressRow, MapResult, ValidateResult};
pub use model_run::{
    ExactRatio, LongestRunResult, MaximumLoadResult, RunCaseResult, RunResult, TargetResult,
    WindowResult,
};
pub use text::{TextRenderError, TextReportStyle, render_text};

pub(crate) fn render_json_with_limit(
    report: &Report,
    limit: usize,
) -> Result<Vec<u8>, JsonRenderError> {
    json::render_json_with_limit(report, limit)
}

pub(crate) fn render_text_with_limit(
    report: &Report,
    style: TextReportStyle,
    limit: usize,
) -> Result<Vec<u8>, TextRenderError> {
    text::render_text_with_limit(report, style, limit)
}
