use serde::{
    Serialize, Serializer,
    ser::{Error as _, SerializeStruct},
};

use crate::{
    issue::{Issue, IssueCode, IssuePath},
    report::{Report, ReportCommand},
};

use super::{JsonRenderError, render_json, render_json_with_limit, serialize_complete};

struct FailsAfterPrefix;

impl Serialize for FailsAfterPrefix {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut report = serializer.serialize_struct("Report", 2)?;
        report.serialize_field("schema_version", &1)?;
        Err(S::Error::custom("injected failure after prefix"))
    }
}

#[test]
fn serialization_failure_exposes_no_partial_buffer() {
    // Given
    let value = FailsAfterPrefix;

    // When
    let result = serialize_complete(&value);

    // Then
    assert!(result.is_err());
}

#[test]
fn json_report_accepts_the_exact_limit_and_rejects_the_next_byte() {
    // Given
    let report = failure_report();
    let expected = render_json(&report).expect("fixture report should render");

    // When
    let exact = render_json_with_limit(&report, expected.len());
    let too_small = render_json_with_limit(&report, expected.len() - 1);

    // Then
    assert_eq!(exact.expect("the exact byte cap should succeed"), expected);
    assert!(matches!(too_small, Err(JsonRenderError::OutputTooLarge(_))));
}

fn failure_report() -> Report {
    Report::failure(
        ReportCommand::Validate,
        vec![Issue::new(
            IssueCode::InputInvalidValue,
            IssuePath::root(),
            "fixture failure",
        )],
    )
    .expect("fixture issue is an error")
}
