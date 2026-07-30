use std::{
    ffi::OsStr,
    io::{self, Write as _},
    path::Path,
};

use crate::{
    cli::OutputFormat,
    input::limits::MAX_REPORT_BYTES,
    io::{
        atomic_output::{
            OutputDestination, OutputError, OutputRequest, format_stderr_issue,
            preflight_report_output, write_report,
        },
        input::InputIdentity,
    },
    issue::{Issue, IssueCode, IssuePath},
    report::{
        JsonRenderError, Report, TextRenderError, TextReportStyle, render_json_with_limit,
        render_text_with_limit,
    },
};

use super::error::{ExecutionError, UsageError};

#[derive(Clone, Copy, Debug)]
pub(super) struct OutputOptions<'path> {
    output: Option<&'path Path>,
    force: bool,
}

impl<'path> OutputOptions<'path> {
    pub(super) const fn new(output: Option<&'path Path>, force: bool) -> Self {
        Self { output, force }
    }

    pub(super) fn validate(self) -> Result<(), UsageError> {
        if self.force && matches!(self.destination(), OutputDestination::Stdout) {
            return Err(UsageError::ForceRequiresPathOutput);
        }
        Ok(())
    }

    pub(super) fn destination(self) -> OutputDestination<'path> {
        match self.output {
            Some(path) if path.as_os_str() != OsStr::new("-") => OutputDestination::Path(path),
            None | Some(_) => OutputDestination::Stdout,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ReportOutput<'path> {
    format: OutputFormat,
    output: OutputOptions<'path>,
    style: TextReportStyle,
}

struct ReportWrite<'report, 'identity> {
    report: &'report Report,
    identities: &'identity [InputIdentity],
    exit: u8,
}

impl<'path> ReportOutput<'path> {
    pub(super) const fn new(
        format: OutputFormat,
        output: OutputOptions<'path>,
        style: TextReportStyle,
    ) -> Self {
        Self {
            format,
            output,
            style,
        }
    }

    pub(super) fn preflight(self, identities: &[InputIdentity]) -> Result<(), ExecutionError> {
        let request = OutputRequest::new(self.output.destination(), self.output.force, identities);
        preflight_report_output(&request).map_err(ExecutionError::from)
    }

    pub(super) fn write(
        self,
        report: &Report,
        identities: &[InputIdentity],
        exit: u8,
    ) -> Result<u8, ExecutionError> {
        let mut stdout = io::stdout().lock();
        self.write_to(
            &ReportWrite {
                report,
                identities,
                exit,
            },
            MAX_REPORT_BYTES,
            &mut stdout,
        )
    }

    fn write_to(
        self,
        write: &ReportWrite<'_, '_>,
        limit: usize,
        stdout: &mut impl io::Write,
    ) -> Result<u8, ExecutionError> {
        let rendered = match self.format {
            OutputFormat::Text => render_text_with_limit(write.report, self.style, limit)
                .map_err(render_text_error)?,
            OutputFormat::Json => {
                render_json_with_limit(write.report, limit).map_err(render_json_error)?
            }
        };
        if self.format == OutputFormat::Text && write.exit != 0 {
            write_business_text(&rendered)?;
            return Ok(write.exit);
        }
        let request = OutputRequest::new(
            self.output.destination(),
            self.output.force,
            write.identities,
        );
        write_report(&request, &rendered, stdout)?;
        Ok(write.exit)
    }
}

fn render_json_error(error: JsonRenderError) -> ExecutionError {
    match error {
        JsonRenderError::OutputTooLarge(_) => OutputError::too_large().into(),
        JsonRenderError::Serialization { .. } => error.into(),
    }
}

fn render_text_error(error: TextRenderError) -> ExecutionError {
    match error {
        TextRenderError::OutputTooLarge(_) => OutputError::too_large().into(),
        TextRenderError::VerboseRequiresValidateCommand | TextRenderError::Formatting => {
            error.into()
        }
    }
}

pub(super) fn write_usage_error(error: UsageError) -> std::process::ExitCode {
    let diagnostic = format!("error: {error}\n");
    let _result = io::stderr().lock().write_all(diagnostic.as_bytes());
    std::process::ExitCode::from(1)
}

pub(super) fn write_execution_error(error: &ExecutionError) {
    let issue = match error {
        ExecutionError::Input(input) | ExecutionError::ScenarioInput(input) => {
            input.issue().clone()
        }
        ExecutionError::Output(output) => output.issue().map_or_else(
            || Issue::new(IssueCode::OutputIo, IssuePath::root(), output.to_string()),
            Clone::clone,
        ),
        ExecutionError::JsonRender(_)
        | ExecutionError::TextRender(_)
        | ExecutionError::ReportModel(_)
        | ExecutionError::Stderr(_) => Issue::new(
            IssueCode::OutputIo,
            IssuePath::root(),
            "report output could not be completed",
        ),
    };
    let _result = io::stderr()
        .lock()
        .write_all(format_stderr_issue(&issue).as_bytes());
}

pub(super) fn write_business_text(bytes: &[u8]) -> Result<(), ExecutionError> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(bytes).map_err(ExecutionError::Stderr)?;
    stderr.flush().map_err(ExecutionError::Stderr)
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
