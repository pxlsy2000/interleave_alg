use std::io;

use thiserror::Error;

use crate::{
    error::ExitClass,
    issue::{Issue, IssueCode, IssuePath},
};

/// A stable usage or filesystem output failure.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct OutputError {
    issue: Option<Issue>,
    #[source]
    source: Option<io::Error>,
    aliases_input: bool,
    message: &'static str,
}

impl OutputError {
    pub(super) const fn usage() -> Self {
        Self {
            issue: None,
            source: None,
            aliases_input: false,
            message: "the --force option requires a path-valued --output",
        }
    }

    pub(super) fn exists(aliases_input: bool) -> Self {
        Self {
            issue: Some(issue(
                IssueCode::OutputExists,
                "output path already exists; use --force to replace it",
            )),
            source: None,
            aliases_input,
            message: "output path already exists; use --force to replace it",
        }
    }

    pub(super) fn invalid_target() -> Self {
        Self {
            issue: Some(issue(
                IssueCode::OutputInvalidTarget,
                "output target must be a regular file",
            )),
            source: None,
            aliases_input: false,
            message: "output target must be a regular file",
        }
    }

    pub(super) fn atomic_unsupported(source: impl Into<io::Error>) -> Self {
        Self {
            issue: Some(issue(
                IssueCode::OutputAtomicUnsupported,
                "atomic no-clobber rename is unsupported",
            )),
            source: Some(source.into()),
            aliases_input: false,
            message: "atomic no-clobber rename is unsupported",
        }
    }

    pub(super) fn io(source: impl Into<io::Error>) -> Self {
        Self {
            issue: Some(issue(
                IssueCode::OutputIo,
                "report output could not be completed",
            )),
            source: Some(source.into()),
            aliases_input: false,
            message: "report output could not be completed",
        }
    }

    pub(crate) fn too_large() -> Self {
        Self {
            issue: Some(issue(
                IssueCode::OutputTooLarge,
                "report exceeds v1 limit 268435456 bytes",
            )),
            source: None,
            aliases_input: false,
            message: "report exceeds v1 limit 268435456 bytes",
        }
    }

    /// Returns the stable issue, or `None` for command-line usage.
    pub const fn issue(&self) -> Option<&Issue> {
        self.issue.as_ref()
    }

    /// Reports whether an existing output was a regular-file input alias.
    pub const fn aliases_input(&self) -> bool {
        self.aliases_input
    }

    /// Returns the process exit class shared by usage and filesystem failures.
    pub const fn exit_class(&self) -> ExitClass {
        ExitClass::UsageOrIo
    }
}

/// Formats one filesystem/input issue for standard error.
pub fn format_stderr_issue(issue: &Issue) -> String {
    let mut diagnostic = format!("ERROR [{}]", issue.code().as_str());
    if !issue.path().as_str().is_empty() {
        diagnostic.push(' ');
        diagnostic.push_str(issue.path().as_str());
    }
    diagnostic.push_str(": ");
    diagnostic.push_str(issue.message());
    diagnostic.push('\n');
    diagnostic
}

fn issue(code: IssueCode, message: &'static str) -> Issue {
    Issue::new(code, IssuePath::root(), message)
}
