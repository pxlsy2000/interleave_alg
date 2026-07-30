//! Linux transactional routing for complete, already-rendered reports.

mod error;
mod linux;
mod temp;

use std::{
    io::{self, Write},
    path::Path,
};

pub use error::{OutputError, format_stderr_issue};

use super::input::InputIdentity;

/// The selected destination for one complete report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputDestination<'path> {
    /// Standard output, covering both omitted `--output` and `--output -`.
    Stdout,
    /// A named filesystem destination.
    Path(&'path Path),
}

/// An injectable failure at one transaction boundary.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AtomicOutputFault {
    /// Execute the real transaction without an injected failure.
    #[default]
    None,
    /// Fail before writing the complete temporary-file payload.
    Write,
    /// Fail after flushing and closing the temporary file.
    Close,
    /// Fail after the temporary file is complete but before commit.
    Precommit,
    /// Simulate `renameat2` returning `ENOSYS`.
    RenameNoReplaceNoSys,
    /// Simulate `renameat2` returning `EOPNOTSUPP`.
    RenameNoReplaceNotSupported,
    /// Simulate 128 consecutive unique-name collisions.
    TempExhaustion,
}

/// Complete output-routing inputs acquired after every bounded input snapshot.
#[derive(Clone, Copy, Debug)]
pub struct OutputRequest<'request> {
    destination: OutputDestination<'request>,
    force: bool,
    input_identities: &'request [InputIdentity],
    fault: AtomicOutputFault,
}

impl<'request> OutputRequest<'request> {
    /// Creates a production output request.
    pub const fn new(
        destination: OutputDestination<'request>,
        force: bool,
        input_identities: &'request [InputIdentity],
    ) -> Self {
        Self {
            destination,
            force,
            input_identities,
            fault: AtomicOutputFault::None,
        }
    }

    /// Selects one deterministic transaction fault for boundary tests.
    #[doc(hidden)]
    #[must_use]
    pub const fn with_fault(mut self, fault: AtomicOutputFault) -> Self {
        self.fault = fault;
        self
    }

    pub(super) const fn force(self) -> bool {
        self.force
    }

    pub(super) const fn input_identities(self) -> &'request [InputIdentity] {
        self.input_identities
    }

    pub(super) const fn fault(self) -> AtomicOutputFault {
        self.fault
    }
}

/// Writes one complete rendered report to stdout or commits it atomically.
///
/// Replacement installs the temporary inode, so prior mode and ownership are
/// not preserved. The transaction intentionally provides no `fsync` guarantee.
pub fn write_report(
    request: &OutputRequest<'_>,
    report: &[u8],
    stdout: &mut impl Write,
) -> Result<(), OutputError> {
    match request.destination {
        OutputDestination::Stdout => write_stdout(*request, report, stdout),
        OutputDestination::Path(path) => linux::write_path(*request, path, report),
    }
}

/// Checks the report destination without creating a temporary file.
///
/// Named destinations are checked again by [`write_report`] immediately before
/// the transaction, so this phase establishes ordering without weakening the
/// commit-time race checks.
pub fn preflight_report_output(request: &OutputRequest<'_>) -> Result<(), OutputError> {
    match request.destination {
        OutputDestination::Stdout => {
            if request.force() {
                Err(OutputError::usage())
            } else {
                Ok(())
            }
        }
        OutputDestination::Path(path) => linux::preflight_path(*request, path),
    }
}

fn write_stdout(
    request: OutputRequest<'_>,
    report: &[u8],
    stdout: &mut impl Write,
) -> Result<(), OutputError> {
    if request.force() {
        return Err(OutputError::usage());
    }
    stdout.write_all(report).map_err(OutputError::io)?;
    stdout.flush().map_err(OutputError::io)
}

pub(super) fn injected_error(boundary: &'static str) -> io::Error {
    io::Error::other(format!("injected {boundary} failure"))
}
