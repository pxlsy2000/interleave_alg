mod format;
mod map;
mod run;
mod validate;

use thiserror::Error;

use crate::input::limits::MAX_REPORT_BYTES;

use super::{
    Report, ReportCommand, ReportResult,
    bounded::{BoundedBytes, OutputTooLarge},
};

/// Requested human-readable report detail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextReportStyle {
    /// Render the ordinary command report.
    Standard,
    /// Include complete Mapping matrices in a validate report.
    Verbose,
}

/// Failure to produce a complete human-readable report.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum TextRenderError {
    /// Verbose matrices apply only to validate reports.
    #[error("verbose text requires a validate report")]
    VerboseRequiresValidateCommand,
    /// The private staging buffer rejected formatted text.
    #[error("text report could not be rendered")]
    Formatting,
    /// The rendered report attempted to exceed the v1 byte cap.
    #[error(transparent)]
    OutputTooLarge(#[from] OutputTooLarge),
}

/// Renders one complete human-readable report with exactly one trailing LF.
///
/// Rendering is staged in a private buffer, so a formatting error never
/// exposes a partial report to the caller.
pub fn render_text(report: &Report, style: TextReportStyle) -> Result<Vec<u8>, TextRenderError> {
    render_text_with_limit(report, style, MAX_REPORT_BYTES)
}

pub(super) fn render_text_with_limit(
    report: &Report,
    style: TextReportStyle,
    limit: usize,
) -> Result<Vec<u8>, TextRenderError> {
    if style == TextReportStyle::Verbose && report.command != ReportCommand::Validate {
        return Err(TextRenderError::VerboseRequiresValidateCommand);
    }
    let mut rendered = BoundedBytes::new(limit);
    let result = match &report.result {
        Some(ReportResult::Validate(result)) => validate::render(
            &mut rendered,
            result,
            &report.warnings,
            &report.errors,
            style,
        ),
        Some(ReportResult::Map(result)) => {
            map::render(&mut rendered, result, &report.warnings, &report.errors)
        }
        Some(ReportResult::Run(result)) => run::render(&mut rendered, result, &report.warnings),
        None => format::render_failure(&mut rendered, report),
    };
    if result.is_err() {
        return if rendered.exceeded() {
            Err(OutputTooLarge.into())
        } else {
            Err(TextRenderError::Formatting)
        };
    }
    rendered.finish().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use crate::issue::{Issue, IssueCode, IssuePath};

    use super::*;

    #[test]
    fn text_report_accepts_the_exact_limit_and_rejects_the_next_byte() {
        // Given
        let report = Report::failure(
            ReportCommand::Validate,
            vec![Issue::new(
                IssueCode::InputInvalidValue,
                IssuePath::root(),
                "fixture failure",
            )],
        )
        .expect("fixture issue is an error");
        let expected =
            render_text(&report, TextReportStyle::Standard).expect("fixture report should render");

        // When
        let exact = render_text_with_limit(&report, TextReportStyle::Standard, expected.len());
        let too_small =
            render_text_with_limit(&report, TextReportStyle::Standard, expected.len() - 1);

        // Then
        assert_eq!(exact.expect("the exact byte cap should succeed"), expected);
        assert!(matches!(too_small, Err(TextRenderError::OutputTooLarge(_))));
    }
}
