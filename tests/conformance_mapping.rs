#![allow(missing_docs)]

#[path = "support/conformance_cli.rs"]
pub mod support;

use support::{TestResult, at, interleave, json};

#[test]
fn g1_n1_preserves_the_complete_line_address_and_zero_offset() -> TestResult {
    // Given
    let mapping = "tests/fixtures/mapping/g1_n1.yaml";

    // When
    let output = interleave(&["map", "--spec", mapping, "0xf", "--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let report = json(&output)?;
    assert_eq!(at(&report, "/result/addresses/0/target")?, 0);
    assert_eq!(at(&report, "/result/addresses/0/line_address")?, "0xf");
    assert_eq!(at(&report, "/result/addresses/0/byte_offset")?, "0x0");
    assert_eq!(
        at(&report, "/result/addresses/0/local_line_address")?,
        "0xf"
    );
    Ok(())
}

#[test]
fn n_equals_two_to_n_has_zero_local_line_at_the_address_maximum() -> TestResult {
    // Given
    let mapping = "tests/fixtures/mapping/full_target_space.yaml";

    // When
    let output = interleave(&["map", "--spec", mapping, "0xf", "--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    let report = json(&output)?;
    assert_eq!(at(&report, "/result/addresses/0/target")?, 15);
    assert_eq!(
        at(&report, "/result/addresses/0/local_line_address")?,
        "0x0"
    );
    assert_eq!(
        at(&report, "/result/addresses/0/local_byte_address")?,
        "0x0"
    );
    Ok(())
}

#[test]
fn a64_maximum_passes_and_exclusive_bound_fails_without_rows() -> TestResult {
    // Given
    let mapping = "tests/fixtures/mapping/a64.yaml";

    // When
    let pass = interleave(&[
        "map",
        "--spec",
        mapping,
        "0xffff_ffff_ffff_ffff",
        "--format",
        "json",
    ])?;
    let fail = interleave(&[
        "map",
        "--spec",
        mapping,
        "0xffffffffffffffff",
        "0x1_0000_0000_0000_0000",
        "--format",
        "json",
    ])?;

    // Then
    assert_eq!(pass.status.code(), Some(0));
    assert_eq!(fail.status.code(), Some(3));
    let failure = json(&fail)?;
    assert_eq!(at(&failure, "/errors/0/code")?, "address.out_of_range");
    assert_eq!(at(&failure, "/result")?, &serde_json::Value::Null);
    Ok(())
}

#[test]
fn duplicate_taps_are_rejected_instead_of_xor_canceled() -> TestResult {
    // Given
    let mapping = "tests/fixtures/invalid/duplicate_tap.yaml";

    // When
    let output = interleave(&["validate", "--spec", mapping, "--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    let report = json(&output)?;
    assert_eq!(at(&report, "/errors/0/code")?, "input.invalid_value");
    assert_eq!(at(&report, "/errors/0/path")?, "mapping.m.rows[0]");
    assert_eq!(
        at(&report, "/errors/0/message")?,
        "expected unique values, observed sequence"
    );
    Ok(())
}

#[test]
fn non_natural_warning_is_retained_by_validate_map_and_run() -> TestResult {
    // Given
    let mapping = "tests/fixtures/mapping/run_non_natural.yaml";
    let commands = [
        vec!["validate", "--spec", mapping, "--format", "json"],
        vec!["map", "--spec", mapping, "0", "--format", "json"],
        vec![
            "run",
            "--spec",
            mapping,
            "--scenario",
            "tests/fixtures/scenario/run_small.yaml",
            "--format",
            "json",
        ],
    ];

    for arguments in commands {
        // When
        let output = interleave(&arguments)?;

        // Then
        assert_eq!(output.status.code(), Some(0));
        let report = json(&output)?;
        assert_eq!(at(&report, "/status")?, "warning");
        assert_eq!(at(&report, "/warnings/0/code")?, "mapping.non_natural");
    }
    Ok(())
}
