mod format;
mod map;
mod run;
mod validate;

use thiserror::Error;

use super::{Report, ReportCommand, ReportResult};

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
}

/// Renders one complete human-readable report with exactly one trailing LF.
///
/// Rendering is staged in a private buffer, so a formatting error never
/// exposes a partial report to the caller.
pub fn render_text(report: &Report, style: TextReportStyle) -> Result<Vec<u8>, TextRenderError> {
    if style == TextReportStyle::Verbose && report.command != ReportCommand::Validate {
        return Err(TextRenderError::VerboseRequiresValidateCommand);
    }
    let mut rendered = String::new();
    match &report.result {
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
    }
    .map_err(|_| TextRenderError::Formatting)?;
    Ok(rendered.into_bytes())
}
