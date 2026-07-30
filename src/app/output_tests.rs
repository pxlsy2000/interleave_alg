use std::fs;

use tempfile::TempDir;

use crate::{
    issue::{Issue, IssueCode, IssuePath},
    report::{ReportCommand, render_json, render_text},
};

use super::*;

#[test]
fn stdout_report_cap_accepts_exact_json_and_rejects_plus_one_without_partial_output() {
    // Given
    let report = failure_report();
    let expected = render_json(&report).expect("fixture report should render");
    let output = ReportOutput::new(
        OutputFormat::Json,
        OutputOptions::new(None, false),
        TextReportStyle::Standard,
    );

    // When
    let mut exact_stdout = Vec::new();
    let exact = output.write_to(
        &ReportWrite {
            report: &report,
            identities: &[],
            exit: 0,
        },
        expected.len(),
        &mut exact_stdout,
    );
    let mut rejected_stdout = Vec::new();
    let rejected = output.write_to(
        &ReportWrite {
            report: &report,
            identities: &[],
            exit: 0,
        },
        expected.len() - 1,
        &mut rejected_stdout,
    );

    // Then
    assert_eq!(exact.expect("the exact byte cap should succeed"), 0);
    assert_eq!(exact_stdout, expected);
    assert!(rejected_stdout.is_empty());
    assert_output_too_large(rejected.expect_err("one byte over the cap must fail"));
}

#[test]
fn file_report_cap_accepts_exact_text_and_rejects_plus_one_atomically() {
    // Given
    let directory = TempDir::new().expect("temporary directory should be available");
    let exact_path = directory.path().join("exact.txt");
    let preserved_path = directory.path().join("preserved.txt");
    let sentinel = b"existing report";
    fs::write(&preserved_path, sentinel).expect("sentinel should be writable");
    let report = failure_report();
    let expected =
        render_text(&report, TextReportStyle::Standard).expect("fixture report should render");

    // When
    let mut exact_stdout = Vec::new();
    let exact = ReportOutput::new(
        OutputFormat::Text,
        OutputOptions::new(Some(&exact_path), false),
        TextReportStyle::Standard,
    )
    .write_to(
        &ReportWrite {
            report: &report,
            identities: &[],
            exit: 0,
        },
        expected.len(),
        &mut exact_stdout,
    );
    let mut rejected_stdout = Vec::new();
    let rejected = ReportOutput::new(
        OutputFormat::Text,
        OutputOptions::new(Some(&preserved_path), true),
        TextReportStyle::Standard,
    )
    .write_to(
        &ReportWrite {
            report: &report,
            identities: &[],
            exit: 0,
        },
        expected.len() - 1,
        &mut rejected_stdout,
    );

    // Then
    assert_eq!(exact.expect("the exact byte cap should succeed"), 0);
    assert!(exact_stdout.is_empty());
    assert_eq!(
        fs::read(&exact_path).expect("exact report should exist"),
        expected
    );
    assert!(rejected_stdout.is_empty());
    assert_output_too_large(rejected.expect_err("one byte over the cap must fail"));
    assert_eq!(
        fs::read(&preserved_path).expect("sentinel should remain readable"),
        sentinel
    );
    let mut names = fs::read_dir(directory.path())
        .expect("temporary directory should be readable")
        .map(|entry| {
            entry
                .expect("directory entry should be readable")
                .file_name()
        })
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names, ["exact.txt", "preserved.txt"]);
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

fn assert_output_too_large(error: ExecutionError) {
    assert_eq!(error.exit_code(), 1);
    let ExecutionError::Output(error) = error else {
        panic!("expected an output error, got {error:?}");
    };
    let issue = error.issue().expect("output limit error has an issue");
    assert_eq!(issue.code(), IssueCode::OutputTooLarge);
    assert_eq!(issue.path().as_str(), "");
    assert_eq!(issue.message(), "report exceeds v1 limit 268435456 bytes");
    assert_eq!(
        format_stderr_issue(issue),
        "ERROR [output.too_large]: report exceeds v1 limit 268435456 bytes\n"
    );
}
