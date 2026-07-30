#![allow(missing_docs)]

use std::{
    fmt::Write as _,
    fs,
    io::Write as _,
    path::Path,
    process::{Command, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const NATURAL_MAPPING: &str = r"schema_version: 1
name: run-map
address:
  width_bits: 20
  granule_bytes: 64
targets:
  count: 4
mapping:
  m:
    rows:
      - [0, 4, 8]
      - [1, 5, 9]
  l:
    mode: preserve_high
";

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

fn interleave(arguments: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(assert_cmd::cargo::cargo_bin!("interleave"))
        .args(arguments)
        .output()
}

fn interleave_stdin(arguments: &[&str], input: &[u8]) -> std::io::Result<std::process::Output> {
    let mut child = Command::new(assert_cmd::cargo::cargo_bin!("interleave"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| std::io::Error::other("child standard input must be piped"))?;
    stdin.write_all(input)?;
    drop(stdin);
    child.wait_with_output()
}

fn write_source(directory: &TempDir, name: &str, body: &str) -> std::io::Result<String> {
    let path = directory.path().join(name);
    fs::write(&path, body)?;
    Ok(path.to_string_lossy().into_owned())
}

fn run_json(
    mapping: &str,
    scenario: &str,
    cases: &[&str],
) -> std::io::Result<std::process::Output> {
    let mut arguments = vec![
        "run",
        "--spec",
        mapping,
        "--scenario",
        scenario,
        "--format",
        "json",
    ];
    for case in cases {
        arguments.extend(["--case", case]);
    }
    interleave(&arguments)
}

fn json_at<'value>(value: &'value Value, pointer: &str) -> std::io::Result<&'value Value> {
    value
        .pointer(pointer)
        .ok_or_else(|| std::io::Error::other(format!("JSON pointer {pointer} must exist")))
}

fn write_fixture_pair(
    directory: &TempDir,
    mapping: &str,
    scenario: &str,
) -> std::io::Result<(String, String)> {
    Ok((
        write_source(directory, "mapping.yaml", mapping)?,
        write_source(directory, "scenario.yaml", scenario)?,
    ))
}

fn assert_untouched(path: &Path, expected: &[u8]) -> TestResult {
    assert_eq!(fs::read(path)?, expected);
    Ok(())
}

include!("support/run_cli_core.rs");
include!("support/run_cli_failures.rs");
include!("support/run_cli_input_routing.rs");
include!("support/run_cli_limits.rs");
include!("support/run_cli_warning_failures.rs");
