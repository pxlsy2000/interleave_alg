use std::{
    ffi::OsStr,
    io::{self, Write as _},
    path::Path,
};

use crate::{
    io::atomic_output::{OutputDestination, format_stderr_issue},
    issue::{Issue, IssueCode, IssuePath},
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

pub(super) fn write_usage_error(error: UsageError) -> std::process::ExitCode {
    let diagnostic = format!("error: {error}\n");
    let _result = io::stderr().lock().write_all(diagnostic.as_bytes());
    std::process::ExitCode::from(1)
}

pub(super) fn write_execution_error(error: &ExecutionError) {
    let issue = match error {
        ExecutionError::Input(input) => input.issue().clone(),
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
