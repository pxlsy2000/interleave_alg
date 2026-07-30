#![allow(missing_docs)]

use std::{fs, process::Command};

use interleave::{
    cli::{Cli, CliCommand},
    input::{limits::MAX_QUERY_ADDRESSES, preflight_query_addresses, scalar::AddressWidth},
    issue::IssueCode,
};
use serde_json::Value;
use tempfile::TempDir;

const NATURAL_MAPPING_PATH: &str = "assets/templates/mapping.yaml";
const NON_NATURAL_MAPPING: &str = r"schema_version: 1
name: non-natural
address:
  width_bits: 8
  granule_bytes: 64
targets:
  count: 2
mapping:
  m:
    rows:
      - [1]
  l:
    mode: explicit
    rows:
      - [0]
";
const NON_BIJECTIVE_MAPPING: &str = r"schema_version: 1
name: broken
address:
  width_bits: 8
  granule_bytes: 64
targets:
  count: 2
mapping:
  m:
    rows:
      - [0]
  l:
    mode: explicit
    rows:
      - [0]
";

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn interleave(arguments: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(assert_cmd::cargo::cargo_bin!("interleave"))
        .args(arguments)
        .output()
}

fn write_mapping(directory: &TempDir, name: &str, body: &str) -> std::io::Result<String> {
    let path = directory.path().join(name);
    fs::write(&path, body)?;
    Ok(path.to_string_lossy().into_owned())
}

fn json_at<'value>(value: &'value Value, pointer: &str) -> std::io::Result<&'value Value> {
    value
        .pointer(pointer)
        .ok_or_else(|| std::io::Error::other(format!("JSON pointer {pointer} must exist")))
}

include!("support/map_cli_core.rs");
include!("support/map_cli_boundaries.rs");
include!("support/map_cli_magnitudes.rs");
include!("support/map_cli_failures.rs");
include!("support/map_cli_routing.rs");
