#![allow(missing_docs)]

#[path = "support/conformance_cli.rs"]
pub mod support;

use support::{TestResult, at, interleave, json};

const MAPPING: &str = "tests/fixtures/mapping/metric_target4.yaml";

#[test]
fn non_power_of_two_targets_are_rejected_without_fallback() -> TestResult {
    // Given
    let source = "tests/fixtures/invalid/nonpower_target.yaml";

    // When
    let output = interleave(&["validate", "--spec", source, "--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    let report = json(&output)?;
    assert_eq!(at(&report, "/errors/0/code")?, "mapping.unsupported");
    assert_eq!(at(&report, "/errors/0/path")?, "targets.count");
    assert_eq!(at(&report, "/result")?, &serde_json::Value::Null);
    Ok(())
}

#[test]
fn unequal_capacity_and_hole_fields_are_both_unknown() -> TestResult {
    // Given
    let excluded = [
        (
            "tests/fixtures/invalid/capacity.yaml",
            "targets[\"capacity\"]",
            "capacity",
        ),
        (
            "tests/fixtures/invalid/holes.yaml",
            "address[\"holes\"]",
            "holes",
        ),
    ];

    for (source, path, field) in excluded {
        // When
        let output = interleave(&["validate", "--spec", source, "--format", "json"])?;

        // Then
        assert_eq!(output.status.code(), Some(2));
        let report = json(&output)?;
        assert_eq!(at(&report, "/errors/0/code")?, "input.unknown_field");
        assert_eq!(at(&report, "/errors/0/path")?, path);
        assert_eq!(at(&report, "/result")?, &serde_json::Value::Null);
        assert!(!String::from_utf8(output.stdout)?.contains(&format!("\"{field}\":")));
    }
    Ok(())
}

#[test]
fn json_and_toml_roots_never_fall_back_to_other_decoders() -> TestResult {
    // Given
    let json_source = "tests/fixtures/invalid/root.json";
    let toml_source = "tests/fixtures/invalid/root.toml";

    // When
    let json_output = interleave(&["validate", "--spec", json_source, "--format", "json"])?;
    let toml_output = interleave(&["validate", "--spec", toml_source, "--format", "json"])?;

    // Then
    assert_eq!(json_output.status.code(), Some(2));
    let json_report = json(&json_output)?;
    assert_eq!(at(&json_report, "/errors/0/code")?, "input.yaml_parse");
    assert_eq!(at(&json_report, "/result")?, &serde_json::Value::Null);

    assert_eq!(toml_output.status.code(), Some(2));
    let toml_report = json(&toml_output)?;
    assert_eq!(at(&toml_report, "/errors/0/code")?, "input.invalid_value");
    assert_eq!(at(&toml_report, "/errors/0/path")?, "");
    assert_eq!(
        at(&toml_report, "/errors/0/message")?,
        "expected mapping, observed \"schema_version=1 name=\\\"t\\\" [address] width_bits=1 granule_bytes=1 [targets] count=1 [mapping] m.rows=[] l.mode=\\\"preserve_high\\\"\""
    );
    assert_eq!(at(&toml_report, "/result")?, &serde_json::Value::Null);
    Ok(())
}

#[test]
fn trace_import_command_and_scenario_field_are_both_absent() -> TestResult {
    // Given
    let scenario = "tests/fixtures/invalid/trace_field.yaml";

    // When
    let help = interleave(&["--help"])?;
    let command = interleave(&["import-trace"])?;
    let field = interleave(&[
        "run",
        "--spec",
        MAPPING,
        "--scenario",
        scenario,
        "--format",
        "json",
    ])?;

    // Then
    let help_text = String::from_utf8(help.stdout)?.to_lowercase();
    assert!(!help_text.contains("trace"));
    assert!(!help_text.contains("import"));
    assert_eq!(command.status.code(), Some(1));
    assert!(command.stdout.is_empty());
    assert!(String::from_utf8(command.stderr)?.contains("unrecognized subcommand"));
    assert_eq!(field.status.code(), Some(3));
    let report = json(&field)?;
    assert_eq!(at(&report, "/errors/0/code")?, "input.unknown_field");
    assert_eq!(at(&report, "/errors/0/path")?, "cases[0][\"trace\"]");
    assert_eq!(at(&report, "/result")?, &serde_json::Value::Null);
    Ok(())
}

#[test]
fn non_round_robin_schedule_is_rejected_without_substitution() -> TestResult {
    // Given
    let scenario = "tests/fixtures/invalid/non_round_robin.yaml";

    // When
    let output = interleave(&[
        "run",
        "--spec",
        MAPPING,
        "--scenario",
        scenario,
        "--format",
        "json",
    ])?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    let report = json(&output)?;
    assert_eq!(at(&report, "/errors/0/code")?, "scenario.invalid");
    assert_eq!(at(&report, "/errors/0/path")?, "cases[0].schedule");
    assert_eq!(
        at(&report, "/errors/0/message")?,
        "expected one of [\"round_robin\"], observed \"weighted\""
    );
    assert_eq!(at(&report, "/result")?, &serde_json::Value::Null);
    Ok(())
}

#[test]
fn aggregate_sweep_field_and_summary_are_both_absent() -> TestResult {
    // Given
    let excluded = "tests/fixtures/invalid/aggregate_sweep.yaml";

    // When
    let rejected = interleave(&[
        "run",
        "--spec",
        MAPPING,
        "--scenario",
        excluded,
        "--format",
        "json",
    ])?;
    let json_output = interleave(&[
        "run",
        "--spec",
        "assets/templates/mapping.yaml",
        "--scenario",
        "assets/templates/scenario.yaml",
        "--case",
        "stride-and-phase-sweep",
        "--format",
        "json",
    ])?;
    let text_output = interleave(&[
        "run",
        "--spec",
        "assets/templates/mapping.yaml",
        "--scenario",
        "assets/templates/scenario.yaml",
        "--case",
        "stride-and-phase-sweep",
    ])?;

    // Then
    assert_eq!(rejected.status.code(), Some(3));
    let failure = json(&rejected)?;
    assert_eq!(at(&failure, "/errors/0/code")?, "input.unknown_field");
    assert_eq!(at(&failure, "/errors/0/path")?, "cases[0][\"aggregate\"]");
    assert_eq!(at(&failure, "/result")?, &serde_json::Value::Null);
    let valid = json(&json_output)?;
    assert_eq!(
        at(&valid, "/result/cases")?.as_array().map(Vec::len),
        Some(12)
    );
    assert!(!String::from_utf8(json_output.stdout)?.contains("\"aggregate\""));
    assert!(!String::from_utf8(text_output.stdout)?.contains("Aggregate"));
    Ok(())
}
