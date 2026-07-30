#![allow(missing_docs)]

use interleave::{
    issue::{Issue, IssueCode, IssueOrderKey, IssuePath, IssuePhase},
    report::{Report, ReportCommand, TextReportStyle, render_text},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "support/text_fixture.rs"]
pub mod fixture;
#[path = "support/text_parity.rs"]
mod text_parity;

#[test]
fn source_ordered_structural_failure_matches_golden() -> TestResult {
    // Given
    let report = Report::failure(
        ReportCommand::Validate,
        vec![
            Issue::new(
                IssueCode::InputUnknownField,
                IssuePath::root().raw_key("later"),
                "unknown field \"later\"",
            )
            .with_order(IssueOrderKey::new(IssuePhase::Schema, 1, 0)),
            Issue::new(
                IssueCode::InputInvalidValue,
                IssuePath::root().field("address").field("width_bits"),
                "expected integer in [1,64], observed 0",
            )
            .with_order(IssueOrderKey::new(IssuePhase::Schema, 0, 0)),
        ],
    )?;

    // When
    let rendered = render_text(&report, TextReportStyle::Standard)?;
    let verbose = render_text(&report, TextReportStyle::Verbose)?;

    // Then
    assert_golden(
        &rendered,
        include_str!("golden/text/validate_structural_failure.txt"),
    )?;
    assert_eq!(verbose, rendered);
    Ok(())
}

#[test]
fn every_validate_classification_matches_golden() -> TestResult {
    // Given
    let cases = [
        (
            fixture::validation_report(fixture::ValidationFixture::Natural)?,
            include_str!("golden/text/validate_natural.txt"),
        ),
        (
            fixture::validation_report(fixture::ValidationFixture::NonNatural)?,
            include_str!("golden/text/validate_non_natural.txt"),
        ),
        (
            fixture::validation_report(fixture::ValidationFixture::TargetUnreachable)?,
            include_str!("golden/text/validate_target_unreachable.txt"),
        ),
        (
            fixture::validation_report(fixture::ValidationFixture::NonBijective)?,
            include_str!("golden/text/validate_non_bijective.txt"),
        ),
    ];

    // When / Then
    for (report, expected) in cases {
        let rendered = render_text(&report, TextReportStyle::Standard)?;
        assert_golden(&rendered, expected)?;
    }
    Ok(())
}

#[test]
fn verbose_nonzero_and_zero_matrices_match_goldens() -> TestResult {
    // Given
    let natural = fixture::validation_report(fixture::ValidationFixture::Natural)?;
    let zero = fixture::zero_dimension_validation_report()?;
    let zero_rows = fixture::zero_target_rows_validation_report()?;

    // When
    let natural_text = render_text(&natural, TextReportStyle::Verbose)?;
    let zero_text = render_text(&zero, TextReportStyle::Verbose)?;
    let zero_rows_text = render_text(&zero_rows, TextReportStyle::Verbose)?;

    // Then
    assert_golden(
        &natural_text,
        include_str!("golden/text/validate_natural_verbose.txt"),
    )?;
    assert_golden(
        &zero_text,
        include_str!("golden/text/validate_zero_verbose.txt"),
    )?;
    assert_golden(
        &zero_rows_text,
        include_str!("golden/text/validate_zero_target_rows_verbose.txt"),
    )?;
    Ok(())
}

#[test]
fn multiple_map_rows_and_warning_match_goldens() -> TestResult {
    // Given
    let natural = fixture::map_report(fixture::ValidationFixture::Natural)?;
    let warning = fixture::map_report(fixture::ValidationFixture::NonNatural)?;

    // When / Then
    assert_golden(
        &render_text(&natural, TextReportStyle::Standard)?,
        include_str!("golden/text/map_multiple.txt"),
    )?;
    assert_golden(
        &render_text(&warning, TextReportStyle::Standard)?,
        include_str!("golden/text/map_non_natural.txt"),
    )?;
    Ok(())
}

#[test]
fn complete_run_and_preflight_failure_match_goldens() -> TestResult {
    // Given
    let complete = fixture::run_report()?;
    let failure = Report::failure(
        ReportCommand::Run,
        vec![Issue::new(
            IssueCode::ScenarioInvalid,
            IssuePath::root()
                .field("cases")
                .index(1)
                .field("window_sizes"),
            "expected every window size <= Q=4, observed sequence",
        )],
    )?;

    // When / Then
    assert_golden(
        &render_text(&complete, TextReportStyle::Standard)?,
        include_str!("golden/text/run_complete.txt"),
    )?;
    assert_golden(
        &render_text(&failure, TextReportStyle::Standard)?,
        include_str!("golden/text/run_preflight_failure.txt"),
    )?;
    Ok(())
}

#[test]
fn map_preflight_failure_has_no_partial_table() -> TestResult {
    // Given
    let report = Report::failure(
        ReportCommand::Map,
        vec![Issue::new(
            IssueCode::AddressOutOfRange,
            IssuePath::root().field("addresses").index(1),
            "address 0x100000 is outside the 20-bit range",
        )],
    )?;

    // When
    let rendered = render_text(&report, TextReportStyle::Standard)?;

    // Then
    assert_golden(
        &rendered,
        include_str!("golden/text/map_preflight_failure.txt"),
    )?;
    assert!(!rendered.windows(7).any(|bytes| bytes == b"Address"));
    Ok(())
}

#[test]
fn text_is_control_free_deterministic_and_single_lf_terminated() -> TestResult {
    // Given
    let report = fixture::validation_report_with_input("\u{1b}[31m mapping.yaml")?;

    // When
    let first = render_text(&report, TextReportStyle::Standard)?;
    let second = render_text(&report, TextReportStyle::Standard)?;

    // Then
    assert_eq!(first, second);
    assert!(first.ends_with(b"\n"));
    assert!(!first.ends_with(b"\n\n"));
    assert!(!first.contains(&0x1b));
    assert!(!first.windows(2).any(|bytes| bytes == b"\r\n"));
    assert!(first.windows(6).any(|bytes| bytes == b"\\u001b"));
    Ok(())
}

fn assert_golden(actual: &[u8], expected: &str) -> TestResult {
    assert_eq!(std::str::from_utf8(actual)?, expected);
    Ok(())
}
