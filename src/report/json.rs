use serde::Serialize;
use thiserror::Error;

use super::Report;

/// Failure to produce a complete canonical JSON document.
#[derive(Debug, Error)]
#[error("JSON report could not be rendered")]
pub struct JsonRenderError {
    #[source]
    source: serde_json::Error,
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
    let mut rendered =
        serde_json::to_vec_pretty(value).map_err(|source| JsonRenderError { source })?;
    rendered.push(b'\n');
    Ok(rendered)
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
