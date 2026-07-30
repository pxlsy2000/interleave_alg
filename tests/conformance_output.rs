#![allow(missing_docs)]

#[path = "support/conformance_cli.rs"]
pub mod support;

use std::fs;

use support::{TestResult, at, interleave, json};
use tempfile::TempDir;

const MAPPING: &str = "tests/fixtures/mapping/metric_target4.yaml";
const SCENARIO: &str = "tests/fixtures/scenario/metric_ties.yaml";

#[test]
fn text_and_json_tie_reports_match_reviewed_goldens_byte_for_byte() -> TestResult {
    // Given
    let common = [
        "run",
        "--spec",
        MAPPING,
        "--scenario",
        SCENARIO,
        "--case",
        "every-tie",
    ];

    // When
    let text_first = interleave(&common)?;
    let text_second = interleave(&common)?;
    let mut json_arguments = common.to_vec();
    json_arguments.extend(["--format", "json"]);
    let json_first = interleave(&json_arguments)?;
    let json_second = interleave(&json_arguments)?;

    // Then
    assert_eq!(text_first.status.code(), Some(0));
    assert_eq!(json_first.status.code(), Some(0));
    assert_eq!(text_first.stdout, text_second.stdout);
    assert_eq!(json_first.stdout, json_second.stdout);
    assert_eq!(
        text_first.stdout,
        include_bytes!("golden/text/conformance_ties.txt")
    );
    assert_eq!(
        json_first.stdout,
        include_bytes!("golden/json/conformance_ties.json")
    );
    assert!(text_first.stdout.ends_with(b"\n"));
    assert!(json_first.stdout.ends_with(b"\n"));
    let report = json(&json_first)?;
    assert_eq!(at(&report, "/result/cases/0/max_load/target")?, 0);
    Ok(())
}

#[test]
fn text_parse_schema_math_and_preflight_failures_use_stderr_only() -> TestResult {
    // Given
    let cases = [
        (
            vec!["validate", "--spec", "tests/fixtures/invalid/root.json"],
            2,
            "input.yaml_parse",
        ),
        (
            vec![
                "validate",
                "--spec",
                "tests/fixtures/invalid/duplicate_tap.yaml",
            ],
            2,
            "input.invalid_value",
        ),
        (
            vec![
                "validate",
                "--spec",
                "tests/fixtures/invalid/non_bijective.yaml",
            ],
            2,
            "mapping.non_bijective",
        ),
        (
            vec![
                "run",
                "--spec",
                "tests/fixtures/mapping/a64.yaml",
                "--scenario",
                "tests/fixtures/invalid/generated_carry.yaml",
            ],
            3,
            "address.out_of_range",
        ),
    ];

    for (arguments, exit, code) in cases {
        // When
        let output = interleave(&arguments)?;

        // Then
        assert_eq!(output.status.code(), Some(exit));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr)?;
        assert!(stderr.contains(&format!("ERROR [{code}]")));
        assert!(stderr.ends_with('\n'));
    }
    Ok(())
}

#[test]
fn json_business_failure_is_a_complete_envelope_on_stdout() -> TestResult {
    // Given
    let mapping = "tests/fixtures/invalid/non_bijective.yaml";

    // When
    let output = interleave(&["validate", "--spec", mapping, "--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert!(output.stdout.ends_with(b"\n"));
    let report = json(&output)?;
    assert_eq!(at(&report, "/status")?, "fail");
    assert_eq!(at(&report, "/errors/0/code")?, "mapping.non_bijective");
    assert_eq!(
        at(&report, "/result/classification")?,
        "invalid_non_bijective"
    );
    Ok(())
}

#[test]
fn preflight_failure_leaves_existing_text_output_and_directory_untouched() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let destination = directory.path().join("report.txt");
    let original = b"existing report\n";
    fs::write(&destination, original)?;
    let destination_text = destination.to_string_lossy().into_owned();

    // When
    let output = interleave(&[
        "run",
        "--spec",
        "tests/fixtures/mapping/a64.yaml",
        "--scenario",
        "tests/fixtures/invalid/generated_carry.yaml",
        "--output",
        &destination_text,
        "--force",
    ])?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(fs::read(&destination)?, original);
    let entries = fs::read_dir(directory.path())?
        .map(|entry| entry.map(|value| value.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(entries, [std::ffi::OsString::from("report.txt")]);
    Ok(())
}

#[test]
fn filesystem_failure_never_emits_a_json_report() -> TestResult {
    // Given
    let missing = "tests/fixtures/invalid/does-not-exist.yaml";

    // When
    let output = interleave(&["validate", "--spec", missing, "--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("ERROR [input.io]"));
    assert!(stderr.ends_with('\n'));
    Ok(())
}
