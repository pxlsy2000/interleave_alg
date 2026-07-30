//! Actual-binary template generation and destination tests.

use std::{fs, process::Command};

use serde_json::Value;
use tempfile::TempDir;

const MAPPING_TEMPLATE: &str = r"# Interleave Mapping template (schema v1).
# XOR tap indices refer to line-address bits x0, x1, ... from least significant upward.
schema_version: 1
name: example-4-target

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
    # preserve_high keeps naturally ordered local-address bits.
    mode: preserve_high
";

const SCENARIO_TEMPLATE: &str = r"# Interleave Scenario template (schema v1).
# Every effective window size is measured in accesses and must not exceed its test length.
schema_version: 1

defaults:
  accesses: 4096
  window_sizes: [4, 16, 64]

cases:
  - name: sequential
    enabled: true
    kind: stride
    base_bytes: 0x0
    stride_bytes: 64

  - name: stride-and-phase-sweep
    enabled: true
    kind: sweep
    base_bytes: [0x0, 0x40, 0x80, 0xc0]
    stride_bytes: [64, 128, 256]
    accesses: 4096
    window_sizes: [4, 16, 64, 256]

  - name: two-master
    enabled: true
    kind: multi_stream
    # round_robin takes one address from each active stream in declaration order.
    schedule: round_robin
    window_sizes: [4, 16, 64]
    streams:
      - name: master0
        base_bytes: 0x0
        stride_bytes: 256
        accesses: 2048
      - name: master1
        base_bytes: 0x40
        stride_bytes: 256
        accesses: 2048
";

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn interleave(arguments: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(assert_cmd::cargo::cargo_bin!("interleave"))
        .args(arguments)
        .output()
}

#[test]
fn mapping_and_scenario_templates_match_the_normative_bytes_on_stdout() -> TestResult {
    // Given
    let cases = [
        (["template", "mapping", "--output", "-"], MAPPING_TEMPLATE),
        (["template", "scenario", "--output", "-"], SCENARIO_TEMPLATE),
    ];

    // When / Then
    for (arguments, expected) in cases {
        let output = interleave(&arguments)?;
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, expected.as_bytes());
        assert!(output.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn template_file_refuses_existing_destination_then_force_replaces_it() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let output_path = directory.path().join("mapping.yaml");
    fs::write(&output_path, b"original\n")?;
    let path = output_path.to_string_lossy();

    // When
    let refused = interleave(&["template", "mapping", "--output", &path])?;
    let forced = interleave(&["template", "mapping", "--output", &path, "--force"])?;

    // Then
    assert_eq!(refused.status.code(), Some(1));
    assert!(refused.stdout.is_empty());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("[output.exists]"));
    assert_eq!(forced.status.code(), Some(0));
    assert!(forced.stdout.is_empty());
    assert!(forced.stderr.is_empty());
    assert_eq!(fs::read(&output_path)?, MAPPING_TEMPLATE.as_bytes());
    Ok(())
}

#[test]
fn template_rejects_an_inaccessible_output_parent() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let missing = directory.path().join("missing/out.yaml");
    let path = missing.to_string_lossy();

    // When
    let output = interleave(&["template", "scenario", "--output", &path])?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("[output.io]"));
    Ok(())
}

#[test]
fn generated_templates_feed_the_mapping_and_scenario_preflight_unchanged() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping_output = interleave(&["template", "mapping", "--output", "-"])?;
    let scenario_output = interleave(&["template", "scenario", "--output", "-"])?;
    assert_eq!(mapping_output.status.code(), Some(0));
    assert_eq!(scenario_output.status.code(), Some(0));
    let mapping_path = directory.path().join("mapping.yaml");
    let scenario_path = directory.path().join("scenario.yaml");
    fs::write(&mapping_path, &mapping_output.stdout)?;
    fs::write(&scenario_path, &scenario_output.stdout)?;
    let mapping_text = mapping_path.to_string_lossy();
    let scenario_text = scenario_path.to_string_lossy();

    // When
    let validation = interleave(&["validate", "--spec", &mapping_text, "--format", "json"])?;
    let run = interleave(&[
        "run",
        "--spec",
        &mapping_text,
        "--scenario",
        &scenario_text,
        "--format",
        "json",
    ])?;

    // Then
    assert_eq!(validation.status.code(), Some(0));
    assert!(validation.stderr.is_empty());
    let validation_report: Value = serde_json::from_slice(&validation.stdout)?;
    assert_eq!(
        validation_report.pointer("/result/classification"),
        Some(&Value::String("valid_natural".to_owned()))
    );
    assert_eq!(run.status.code(), Some(0));
    assert!(run.stderr.is_empty());
    let run_report: Value = serde_json::from_slice(&run.stdout)?;
    assert_eq!(
        run_report
            .pointer("/result/cases")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(14)
    );
    Ok(())
}
