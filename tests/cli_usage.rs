//! Actual-binary command grammar and usage-routing tests.

use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn interleave(arguments: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(assert_cmd::cargo::cargo_bin!("interleave"))
        .args(arguments)
        .output()
}

fn assert_usage_failure(arguments: &[&str]) -> TestResult {
    // Given
    let invocation = arguments;

    // When
    let output = interleave(invocation)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    Ok(())
}

#[test]
fn root_help_and_version_use_stdout_when_requested() -> TestResult {
    // Given
    let cases = [["--help"], ["--version"]];

    // When
    let outputs = cases
        .iter()
        .map(|arguments| interleave(arguments))
        .collect::<Result<Vec<_>, _>>()?;

    // Then
    for output in &outputs {
        assert_eq!(output.status.code(), Some(0));
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    assert_eq!(
        outputs.get(1).map(|output| output.stdout.as_slice()),
        Some(b"interleave 0.1.0\n".as_slice())
    );
    Ok(())
}

#[test]
fn every_command_help_path_uses_stdout() -> TestResult {
    // Given
    let cases: &[&[&str]] = &[
        &["template", "--help"],
        &["template", "mapping", "--help"],
        &["template", "scenario", "--help"],
        &["validate", "--help"],
    ];

    // When
    let outputs = cases
        .iter()
        .map(|arguments| interleave(arguments))
        .collect::<Result<Vec<_>, _>>()?;

    // Then
    for output in outputs {
        assert_eq!(output.status.code(), Some(0));
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn validate_help_short_circuits_before_attempting_to_read_its_input() -> TestResult {
    // Given
    let arguments = [
        "validate",
        "--spec",
        "/definitely/missing/interleave-mapping.yaml",
        "--help",
    ];

    // When
    let output = interleave(&arguments)?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    assert!(!output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn every_command_version_path_uses_the_authoritative_root_version_line() -> TestResult {
    // Given
    let cases: &[&[&str]] = &[
        &["--version"],
        &["template", "--version"],
        &["template", "mapping", "--version"],
        &["template", "scenario", "--version"],
        &["validate", "--version"],
    ];

    // When
    let outputs = cases
        .iter()
        .map(|arguments| interleave(arguments))
        .collect::<Result<Vec<_>, _>>()?;

    // Then
    for output in outputs {
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, b"interleave 0.1.0\n");
        assert!(output.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn missing_and_extra_arguments_are_usage_failures() -> TestResult {
    // Given
    let cases: &[&[&str]] = &[
        &[],
        &["template"],
        &["template", "mapping"],
        &["validate"],
        &["validate", "--spec", "missing.yaml", "extra"],
        &["unknown"],
    ];

    // When / Then
    for arguments in cases {
        assert_usage_failure(arguments)?;
    }
    Ok(())
}

#[test]
fn duplicate_singleton_options_are_usage_failures_before_input_read() -> TestResult {
    // Given
    let cases: &[&[&str]] = &[
        &["validate", "--spec", "a", "--spec", "b"],
        &[
            "validate",
            "--spec",
            "missing.yaml",
            "--format",
            "text",
            "--format",
            "json",
        ],
        &[
            "validate",
            "--spec",
            "missing.yaml",
            "--output",
            "a",
            "--output",
            "b",
        ],
        &[
            "validate",
            "--spec",
            "missing.yaml",
            "--verbose",
            "--verbose",
        ],
        &["validate", "--spec", "missing.yaml", "--force", "--force"],
        &["template", "mapping", "--output", "a", "--output", "b"],
        &["template", "mapping", "--output", "a", "--force", "--force"],
    ];

    // When / Then
    for arguments in cases {
        let output = interleave(arguments)?;
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("input.io"));
    }
    Ok(())
}

#[test]
fn cross_option_usage_rules_are_checked_before_input_read() -> TestResult {
    // Given
    let cases: &[&[&str]] = &[
        &[
            "validate",
            "--spec",
            "missing.yaml",
            "--verbose",
            "--format",
            "json",
        ],
        &["validate", "--spec", "missing.yaml", "--force"],
        &[
            "validate",
            "--spec",
            "missing.yaml",
            "--output",
            "-",
            "--force",
        ],
        &["template", "mapping", "--output", "-", "--force"],
        &[
            "template", "mapping", "--output", "out.yaml", "--format", "text",
        ],
    ];

    // When / Then
    for arguments in cases {
        assert_usage_failure(arguments)?;
    }
    Ok(())
}
