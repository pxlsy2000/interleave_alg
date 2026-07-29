//! Exact scalar parsers, operational limits, and YAML ingestion.

pub mod limits;
pub mod scalar;
pub mod yaml;

pub use crate::io::input::{
    InputIdentity, InputReadError, InputSnapshot, read_bounded, read_named, read_stdin,
};
pub use yaml::{
    InputLoadError, ScalarKind, ScalarStyle, SourcePosition, SourceSpan, SpannedMappingEntry,
    SpannedYamlDocument, SpannedYamlKind, SpannedYamlNode, SpannedYamlScalar, load_yaml_bytes,
    load_yaml_named, load_yaml_reader, parse_yaml,
};
