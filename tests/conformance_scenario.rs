#![allow(missing_docs)]

#[path = "support/conformance_cli.rs"]
pub mod support;

use support::{TestResult, at, interleave, json};

const MAPPING: &str = "tests/fixtures/mapping/metric_target4.yaml";

#[test]
fn zero_stride_repeats_one_address_for_the_complete_test() -> TestResult {
    // Given
    let scenario = "tests/fixtures/scenario/zero_stride.yaml";

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
    assert_eq!(output.status.code(), Some(0));
    let report = json(&output)?;
    assert_eq!(at(&report, "/result/cases/0/accesses")?, 4);
    assert_eq!(at(&report, "/result/cases/0/longest_run/length")?, 4);
    assert_eq!(at(&report, "/result/cases/0/longest_run/target")?, 3);
    Ok(())
}

#[test]
fn unequal_and_single_stream_round_robin_keep_declared_order() -> TestResult {
    // Given
    let scenario = "tests/fixtures/scenario/unequal_streams.yaml";

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
    assert_eq!(output.status.code(), Some(0));
    let report = json(&output)?;
    assert_eq!(at(&report, "/result/cases/0/accesses")?, 3);
    assert_eq!(at(&report, "/result/cases/1/accesses")?, 5);
    assert_eq!(at(&report, "/result/cases/1/targets/0/count")?, 2);
    assert_eq!(at(&report, "/result/cases/1/targets/1/count")?, 1);
    assert_eq!(at(&report, "/result/cases/1/targets/2/count")?, 1);
    assert_eq!(at(&report, "/result/cases/1/targets/3/count")?, 1);
    Ok(())
}

#[test]
fn disabled_selection_and_repeated_case_names_are_deterministic() -> TestResult {
    // Given
    let scenario = "tests/fixtures/scenario/disabled_selection.yaml";

    // When
    let defaults = interleave(&[
        "run",
        "--spec",
        MAPPING,
        "--scenario",
        scenario,
        "--format",
        "json",
    ])?;
    let explicit = interleave(&[
        "run",
        "--spec",
        MAPPING,
        "--scenario",
        scenario,
        "--case",
        "enabled",
        "--case",
        "disabled",
        "--case",
        "disabled",
        "--format",
        "json",
    ])?;

    // Then
    let default_report = json(&defaults)?;
    assert_eq!(
        at(&default_report, "/result/cases/0/source_case")?,
        "enabled"
    );
    assert_eq!(
        at(&default_report, "/result/cases")?
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let explicit_report = json(&explicit)?;
    assert_eq!(
        at(&explicit_report, "/result/cases/0/source_case")?,
        "disabled"
    );
    assert_eq!(
        at(&explicit_report, "/result/cases/1/source_case")?,
        "enabled"
    );
    assert_eq!(
        at(&explicit_report, "/result/cases")?
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    Ok(())
}

#[test]
fn all_metric_ties_follow_the_normative_representatives() -> TestResult {
    // Given
    let scenario = "tests/fixtures/scenario/metric_ties.yaml";

    // When
    let output = interleave(&[
        "run",
        "--spec",
        MAPPING,
        "--scenario",
        scenario,
        "--case",
        "every-tie",
        "--format",
        "json",
    ])?;

    // Then
    let report = json(&output)?;
    assert_eq!(at(&report, "/result/cases/0/max_load/target")?, 0);
    assert_eq!(at(&report, "/result/cases/0/windows/0/start_index")?, 0);
    assert_eq!(at(&report, "/result/cases/0/windows/0/target")?, 0);
    assert_eq!(at(&report, "/result/cases/0/longest_run/start_index")?, 0);
    assert_eq!(at(&report, "/result/cases/0/longest_run/target")?, 1);
    assert_eq!(at(&report, "/result/cases/0/targets/2/count")?, 0);
    Ok(())
}

#[test]
fn duplicate_sweep_values_and_generated_carry_fail_before_analysis() -> TestResult {
    // Given
    let duplicate = "tests/fixtures/invalid/duplicate_sweep.yaml";
    let carry = "tests/fixtures/invalid/generated_carry.yaml";

    // When
    let duplicate_output = interleave(&[
        "run",
        "--spec",
        MAPPING,
        "--scenario",
        duplicate,
        "--format",
        "json",
    ])?;
    let carry_output = interleave(&[
        "run",
        "--spec",
        "tests/fixtures/mapping/a64.yaml",
        "--scenario",
        carry,
        "--format",
        "json",
    ])?;

    // Then
    assert_eq!(duplicate_output.status.code(), Some(3));
    let duplicate_report = json(&duplicate_output)?;
    assert_eq!(
        at(&duplicate_report, "/errors/0/message")?,
        "expected unique values, observed sequence"
    );
    assert_eq!(carry_output.status.code(), Some(3));
    let carry_report = json(&carry_output)?;
    assert_eq!(at(&carry_report, "/errors/0/code")?, "address.out_of_range");
    assert_eq!(at(&carry_report, "/result")?, &serde_json::Value::Null);
    Ok(())
}
