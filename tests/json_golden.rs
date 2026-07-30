#![allow(missing_docs)]

use interleave::{
    issue::{Issue, IssueCode, IssuePath},
    report::{Report, ReportCommand, render_json},
};
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "support/json_fixture.rs"]
pub mod fixture;
#[path = "support/json_command_cases.rs"]
mod json_command_cases;
#[path = "support/json_validation_cases.rs"]
mod json_validation_cases;

#[test]
fn structural_failure_has_null_result_when_rendered() -> TestResult {
    // Given
    let report = Report::failure(
        ReportCommand::Validate,
        vec![Issue::new(
            IssueCode::InputInvalidValue,
            IssuePath::root().field("mapping"),
            "expected mapping, observed null",
        )],
    )?;

    // When
    let rendered = render_json(&report)?;

    // Then
    let expected = concat!(
        "{\n",
        "  \"schema_version\": 1,\n",
        "  \"command\": \"validate\",\n",
        "  \"status\": \"fail\",\n",
        "  \"warnings\": [],\n",
        "  \"errors\": [\n",
        "    {\n",
        "      \"code\": \"input.invalid_value\",\n",
        "      \"path\": \"mapping\",\n",
        "      \"message\": \"expected mapping, observed null\"\n",
        "    }\n",
        "  ],\n",
        "  \"result\": null\n",
        "}\n",
    );
    assert_eq!(rendered, expected.as_bytes());
    Ok(())
}

#[test]
fn run_preflight_failure_has_no_partial_cases() -> TestResult {
    // Given
    let report = Report::failure(
        ReportCommand::Run,
        vec![Issue::new(
            IssueCode::ScenarioInvalid,
            IssuePath::root()
                .field("cases")
                .index(1)
                .field("window_sizes"),
            "window size must not exceed accesses",
        )],
    )?;

    // When
    let actual: Value = serde_json::from_slice(&render_json(&report)?)?;

    // Then
    assert_eq!(at(&actual, "/status")?, "fail");
    assert_eq!(
        at(&actual, "/errors/0")?,
        &json!({
            "code": "scenario.invalid",
            "path": "cases[1].window_sizes",
            "message": "window size must not exceed accesses"
        })
    );
    assert!(at(&actual, "/result")?.is_null());
    Ok(())
}

#[test]
fn issue_arrays_preserve_phase_order_and_encoded_paths() -> TestResult {
    // Given
    let report = Report::failure(
        ReportCommand::Validate,
        vec![
            Issue::new(
                IssueCode::InputUnknownField,
                IssuePath::root().raw_key("bad.key"),
                "unknown field",
            ),
            Issue::new(
                IssueCode::InputYamlParse,
                IssuePath::root(),
                "invalid YAML document",
            ),
        ],
    )?;

    // When
    let actual: Value = serde_json::from_slice(&render_json(&report)?)?;

    // Then
    assert_eq!(at(&actual, "/errors/0/code")?, "input.yaml_parse");
    assert_eq!(at(&actual, "/errors/1/code")?, "input.unknown_field");
    assert_eq!(at(&actual, "/errors/1/path")?, "[\"bad.key\"]");
    Ok(())
}

#[test]
fn every_number_is_safe_and_rendering_is_byte_stable() -> TestResult {
    // Given
    let report = fixture::run_report()?;

    // When
    let first = render_json(&report)?;
    let second = render_json(&report)?;
    let parsed: Value = serde_json::from_slice(&first)?;

    // Then
    assert_eq!(first, second);
    assert!(first.ends_with(b"\n"));
    assert!(!first.ends_with(b"\n\n"));
    assert_safe_numbers(&parsed)?;
    Ok(())
}

fn assert_safe_numbers(value: &Value) -> TestResult {
    match value {
        Value::Number(number) => {
            let integer = number
                .as_u64()
                .ok_or("JSON report contains a non-unsigned integer")?;
            if integer > 9_007_199_254_740_991 {
                return Err("JSON report contains an unsafe integer".into());
            }
        }
        Value::Array(values) => {
            for value in values {
                assert_safe_numbers(value)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                assert_safe_numbers(value)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::String(_) => {}
    }
    Ok(())
}

fn at<'value>(value: &'value Value, pointer: &str) -> TestResult<&'value Value> {
    value
        .pointer(pointer)
        .ok_or_else(|| format!("missing JSON pointer {pointer}").into())
}
