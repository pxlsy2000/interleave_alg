use std::io::Write as _;

use serde::Serialize;
use thiserror::Error;

use crate::input::limits::MAX_REPORT_BYTES;

use super::{
    Report,
    bounded::{BoundedBytes, OutputTooLarge},
};

/// Failure to produce a complete bounded canonical JSON document.
#[derive(Debug, Error)]
pub enum JsonRenderError {
    /// JSON serialization failed for a reason other than the report byte cap.
    #[error("JSON report could not be rendered")]
    Serialization {
        /// Original serialization failure.
        #[source]
        source: serde_json::Error,
    },
    /// The rendered report attempted to exceed the v1 byte cap.
    #[error(transparent)]
    OutputTooLarge(#[from] OutputTooLarge),
}

/// Renders one complete pretty JSON document with exactly one trailing LF.
///
/// Serialization is staged in a private buffer, so an error never exposes a
/// partial document to the caller.
///
/// # Errors
/// Returns an error if serialization cannot complete.
pub fn render_json(report: &Report) -> Result<Vec<u8>, JsonRenderError> {
    serialize_complete(report)
}

fn serialize_complete(value: &impl Serialize) -> Result<Vec<u8>, JsonRenderError> {
    serialize_with_limit(value, MAX_REPORT_BYTES)
}

pub(super) fn render_json_with_limit(
    report: &Report,
    limit: usize,
) -> Result<Vec<u8>, JsonRenderError> {
    serialize_with_limit(report, limit)
}

fn serialize_with_limit(value: &impl Serialize, limit: usize) -> Result<Vec<u8>, JsonRenderError> {
    let mut rendered = BoundedBytes::new(limit);
    if let Err(source) = serde_json::to_writer_pretty(&mut rendered, value) {
        if rendered.exceeded() {
            return Err(OutputTooLarge.into());
        }
        return Err(JsonRenderError::Serialization { source });
    }
    if rendered.write_all(b"\n").is_err() {
        return Err(OutputTooLarge.into());
    }
    rendered.finish().map_err(Into::into)
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
