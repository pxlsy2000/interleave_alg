//! Actual-binary template generation and destination tests.

use std::{fs, process::Command};

use interleave::{
    input::load_yaml_bytes,
    mapping::{MappingClassification, decode_mapping, validate_mapping},
    scenario::{decode_scenario, preflight_scenarios, select_cases},
};
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
    let mapping_output = interleave(&["template", "mapping", "--output", "-"])?;
    let scenario_output = interleave(&["template", "scenario", "--output", "-"])?;
    assert_eq!(mapping_output.status.code(), Some(0));
    assert_eq!(scenario_output.status.code(), Some(0));

    // When
    let mapping_document = load_yaml_bytes(&mapping_output.stdout)?;
    let mapping = decode_mapping(&mapping_document)?;
    let validation = validate_mapping(&mapping);
    let scenario_document = load_yaml_bytes(&scenario_output.stdout)?;
    let scenario = decode_scenario(&scenario_document)?;
    let selected = select_cases(&scenario, &[])?;
    let preflight = preflight_scenarios(&mapping, &selected)?;

    // Then
    assert_eq!(
        validation.classification(),
        MappingClassification::ValidNatural
    );
    assert_eq!(preflight.tests().len(), 14);
    Ok(())
}
