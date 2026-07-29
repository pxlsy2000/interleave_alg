//! Strict YAML 1.2 ingestion after bounded source acquisition.

mod build;
mod decode;
mod diagnostic;
mod events;
mod location;
mod path;
mod tree;

use std::{
    io::{Cursor, Read},
    path::Path,
};

use thiserror::Error;

use crate::{
    input::{InputReadError, InputSnapshot, read_bounded, read_named},
    issue::Issue,
};

pub use tree::{
    ScalarKind, ScalarStyle, SourcePosition, SourceSpan, SpannedMappingEntry, SpannedYamlDocument,
    SpannedYamlKind, SpannedYamlNode, SpannedYamlScalar,
};

use self::{build::build_root, diagnostic::YamlParseError, events::collect};

/// A bounded-read or strict-YAML failure carrying exactly one stable issue.
#[derive(Debug, Error)]
pub enum InputLoadError {
    /// Input acquisition failed before YAML inspection.
    #[error(transparent)]
    Read(#[from] InputReadError),
    /// Encoding, syntax, document, or prohibited YAML failed.
    #[error("YAML input is invalid")]
    Yaml {
        /// Stable YAML issue.
        issue: Issue,
        /// Earliest winning source position.
        position: SourcePosition,
    },
}

impl InputLoadError {
    /// Returns the single stable issue produced by the failed phase.
    pub const fn issue(&self) -> &Issue {
        match self {
            Self::Read(error) => error.issue(),
            Self::Yaml { issue, .. } => issue,
        }
    }

    /// Returns the winning YAML source position, when parsing was reached.
    pub const fn position(&self) -> Option<SourcePosition> {
        match self {
            Self::Read(_) => None,
            Self::Yaml { position, .. } => Some(*position),
        }
    }

    /// Returns 16 MiB + 1 when bounded acquisition stopped at the raw cap.
    pub const fn observed_raw_bytes(&self) -> Option<usize> {
        match self {
            Self::Read(error) => error.observed_raw_bytes(),
            Self::Yaml { .. } => None,
        }
    }
}

/// Bounded-reads and parses an in-memory YAML source.
pub fn load_yaml_bytes(source: &[u8]) -> Result<SpannedYamlDocument, InputLoadError> {
    load_yaml_reader(Cursor::new(source))
}

/// Bounded-reads and parses a YAML stream using the common reader.
pub fn load_yaml_reader(reader: impl Read) -> Result<SpannedYamlDocument, InputLoadError> {
    let snapshot = read_bounded(reader)?;
    parse_yaml(&snapshot)
}

/// Bounded-reads and parses a named YAML source while retaining file identity.
pub fn load_yaml_named(
    path: &Path,
) -> Result<(SpannedYamlDocument, InputSnapshot), InputLoadError> {
    let snapshot = read_named(path)?;
    let document = parse_yaml(&snapshot)?;
    Ok((document, snapshot))
}

/// Parses one already bounded and complete source snapshot.
pub fn parse_yaml(snapshot: &InputSnapshot) -> Result<SpannedYamlDocument, InputLoadError> {
    let decoded = decode::decode(snapshot.bytes());
    let mut stream = collect(decoded.text, decoded.bom_bytes);
    stream.violations.extend(decoded.violations);

    let root_index = stream.root_indices.first().copied();
    let root = root_index.and_then(|index| {
        let built = build_root(&stream.events, &stream.positions, index);
        stream.violations.extend(built.violations);
        built.root
    });

    if let Some(error) = YamlParseError::from_violations(stream.violations) {
        return Err(yaml_error(&error));
    }
    if !has_document_content(decoded.text) || stream.document_count == 0 {
        return Err(yaml_error(&YamlParseError::no_document(eof_position(
            decoded.text,
            snapshot.bytes().len(),
        ))));
    }
    root.map(SpannedYamlDocument::new).ok_or_else(|| {
        yaml_error(&YamlParseError::no_document(eof_position(
            decoded.text,
            snapshot.bytes().len(),
        )))
    })
}

fn has_document_content(source: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.trim_start();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    })
}

fn eof_position(source: &str, byte_offset: usize) -> SourcePosition {
    let line = source.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = source
        .rsplit_once('\n')
        .map_or_else(|| source.chars().count(), |(_, tail)| tail.chars().count());
    SourcePosition::new(byte_offset, line, column)
}

fn yaml_error(error: &YamlParseError) -> InputLoadError {
    InputLoadError::Yaml {
        issue: error.issue().clone(),
        position: error.position(),
    }
}
