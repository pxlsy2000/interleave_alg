use thiserror::Error;

use crate::{
    io::{atomic_output::OutputError, input::InputReadError},
    report::{JsonRenderError, ReportModelError, TextRenderError},
};

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(super) enum UsageError {
    #[error("the --verbose option cannot be used with --format json")]
    VerboseJson,
    #[error("the --force option requires a path-valued --output")]
    ForceRequiresPathOutput,
    #[error("the Mapping and Scenario cannot both use standard input")]
    MultipleStdinInputs,
}

#[derive(Debug, Error)]
pub(super) enum ExecutionError {
    #[error(transparent)]
    Input(#[from] InputReadError),
    #[error(transparent)]
    ScenarioInput(InputReadError),
    #[error(transparent)]
    Output(#[from] OutputError),
    #[error(transparent)]
    JsonRender(#[from] JsonRenderError),
    #[error(transparent)]
    TextRender(#[from] TextRenderError),
    #[error(transparent)]
    ReportModel(#[from] ReportModelError),
    #[error("standard error could not be written")]
    Stderr(#[source] std::io::Error),
}

impl ExecutionError {
    pub(super) const fn exit_code(&self) -> u8 {
        match self {
            Self::Input(error) => {
                if error.observed_raw_bytes().is_some() {
                    2
                } else {
                    1
                }
            }
            Self::ScenarioInput(error) => {
                if error.observed_raw_bytes().is_some() {
                    3
                } else {
                    1
                }
            }
            Self::Output(error) => error.exit_class().code(),
            Self::JsonRender(_) | Self::TextRender(_) | Self::ReportModel(_) | Self::Stderr(_) => 1,
        }
    }
}
