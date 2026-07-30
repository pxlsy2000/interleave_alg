use interleave::{
    issue::{Issue, IssueCode, IssuePath},
    report::{Report, ReportCommand, render_json},
};
use serde_json::{Value, json};

use super::{
    TestResult, at,
    fixture::{MappingFixture, map_report, run_report},
};

#[test]
fn map_success_preserves_order_and_canonicalizes_addresses() -> TestResult {
    // Given
    let report = map_report(MappingFixture::Natural, &["0x0_2", "3"])?;

    // When
    let actual: Value = serde_json::from_slice(&render_json(&report)?)?;

    // Then
    assert_eq!(
        at(&actual, "/result/addresses")?,
        &json!([
            {
                "input_address": "0x2",
                "line_address": "0x2",
                "byte_offset": "0x0",
                "target": 2,
                "local_line_address": "0x0",
                "local_byte_address": "0x0"
            },
            {
                "input_address": "0x3",
                "line_address": "0x3",
                "byte_offset": "0x0",
                "target": 3,
                "local_line_address": "0x0",
                "local_byte_address": "0x0"
            }
        ])
    );
    assert_eq!(
        at(&actual, "/result/mapping_classification")?,
        "valid_natural"
    );
    Ok(())
}

#[test]
fn map_warning_and_preflight_failure_have_distinct_result_shapes() -> TestResult {
    // Given
    let warning_report = map_report(MappingFixture::NonNatural, &["0x2"])?;
    let failure_report = Report::failure(
        ReportCommand::Map,
        vec![Issue::new(
            IssueCode::AddressOutOfRange,
            IssuePath::root().field("addresses").index(0),
            "address 0x100 is outside the 8-bit range",
        )],
    )?;

    // When
    let warning: Value = serde_json::from_slice(&render_json(&warning_report)?)?;
    let failure: Value = serde_json::from_slice(&render_json(&failure_report)?)?;

    // Then
    assert_eq!(at(&warning, "/status")?, "warning");
    assert_eq!(
        at(&warning, "/result/mapping_classification")?,
        "valid_non_natural"
    );
    assert!(at(&warning, "/result")?.is_object());
    assert_eq!(at(&failure, "/status")?, "fail");
    assert!(at(&failure, "/result")?.is_null());
    assert_eq!(at(&failure, "/errors/0/path")?, "addresses[0]");
    Ok(())
}

#[test]
fn run_success_matches_ordered_multi_case_golden() -> TestResult {
    // Given
    let report = run_report()?;

    // When
    let actual: Value = serde_json::from_slice(&render_json(&report)?)?;

    // Then
    assert_eq!(at(&actual, "/command")?, "run");
    assert_eq!(at(&actual, "/status")?, "pass");
    assert_eq!(at(&actual, "/result/cases/0/case_id")?, "sequential");
    assert_eq!(at(&actual, "/result/cases/1/case_id")?, "two-stream");
    assert_eq!(
        at(&actual, "/result/cases/0/targets")?,
        &json!([
            {"target": 0, "count": 1, "share": {"numerator": 1, "denominator": 4, "decimal": "0.250000"}},
            {"target": 1, "count": 1, "share": {"numerator": 1, "denominator": 4, "decimal": "0.250000"}},
            {"target": 2, "count": 1, "share": {"numerator": 1, "denominator": 4, "decimal": "0.250000"}},
            {"target": 3, "count": 1, "share": {"numerator": 1, "denominator": 4, "decimal": "0.250000"}}
        ])
    );
    assert_eq!(
        at(&actual, "/result/cases/0/max_load")?,
        &json!({
            "target": 0,
            "count": 1,
            "ratio": {"numerator": 4, "denominator": 4, "decimal": "1.000000"}
        })
    );
    assert_eq!(
        at(&actual, "/result/cases/0/windows")?,
        &json!([{
            "size": 2,
            "target": 0,
            "start_index": 0,
            "count": 1,
            "ratio": {"numerator": 4, "denominator": 2, "decimal": "2.000000"}
        }])
    );
    assert_eq!(
        at(&actual, "/result/cases/1/windows")?,
        &json!([
            {
                "size": 2,
                "target": 0,
                "start_index": 0,
                "count": 1,
                "ratio": {"numerator": 4, "denominator": 2, "decimal": "2.000000"}
            },
            {
                "size": 4,
                "target": 0,
                "start_index": 0,
                "count": 2,
                "ratio": {"numerator": 8, "denominator": 4, "decimal": "2.000000"}
            }
        ])
    );
    assert_eq!(
        at(&actual, "/result/cases/0/longest_run")?,
        &json!({"length": 1, "target": 0, "start_index": 0})
    );
    Ok(())
}
