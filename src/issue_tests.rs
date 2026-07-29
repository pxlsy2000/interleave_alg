use super::{
    CheckStatus, Issue, IssueCode, IssueOrderKey, IssuePath, IssuePhase, ReportStatus, Severity,
    canonical_json_string, report_status, sort_issues,
};

#[test]
fn raw_key_paths_use_canonical_json_bracket_segments() {
    // Given
    let key = "bad.key\"\\\u{0001}\u{007f}\u{0085}\u{2028}\u{2029}";

    // When
    let path = IssuePath::root()
        .field("cases")
        .index(2)
        .field("streams")
        .index(1)
        .raw_key(key);

    // Then
    assert_eq!(
        path.as_str(),
        "cases[2].streams[1][\"bad.key\\\"\\\\\\u0001\\u007f\\u0085\\u2028\\u2029\"]"
    );
    assert_eq!(
        canonical_json_string("\u{0008}\u{000c}\n\r\t/"),
        "\"\\b\\f\\n\\r\\t/\""
    );
}

#[test]
fn issue_sorting_uses_phase_then_source_then_field_order() {
    // Given
    let mut issues = [
        Issue::new(
            IssueCode::ScenarioInvalid,
            IssuePath::root().field("cases").index(2),
            "later",
        )
        .with_order(IssueOrderKey::new(IssuePhase::ScenarioSemantic, 2, 0)),
        Issue::new(
            IssueCode::InputInvalidValue,
            IssuePath::root().field("address"),
            "first",
        )
        .with_order(IssueOrderKey::new(IssuePhase::Schema, 5, 1)),
        Issue::new(
            IssueCode::InputUnknownField,
            IssuePath::root().raw_key("z"),
            "second",
        )
        .with_order(IssueOrderKey::new(IssuePhase::Schema, 5, 2)),
    ];

    // When
    sort_issues(&mut issues);

    // Then
    assert_eq!(
        issues.map(|issue| issue.message().to_owned()),
        ["first", "second", "later"]
    );
}

#[test]
fn report_status_is_deterministic_from_severities() {
    // Given
    let warning = Issue::new(
        IssueCode::MappingNonNatural,
        IssuePath::root().field("mapping"),
        "warning",
    );
    let error = Issue::new(
        IssueCode::MappingNonBijective,
        IssuePath::root().field("mapping").field("l").field("rows"),
        "error",
    );

    // When / Then
    assert_eq!(report_status(&[]), ReportStatus::Pass);
    assert_eq!(
        report_status(std::slice::from_ref(&warning)),
        ReportStatus::Warning
    );
    assert_eq!(report_status(&[warning, error]), ReportStatus::Fail);
    assert_eq!(ReportStatus::Pass.as_str(), "pass");
    assert_eq!(ReportStatus::Warning.as_str(), "warning");
    assert_eq!(ReportStatus::Fail.as_str(), "fail");
    assert_eq!(IssueCode::MappingNonNatural.severity(), Severity::Warning);
    assert_eq!(IssueCode::AddressInvalid.severity(), Severity::Error);
    assert_eq!(CheckStatus::Pass.as_str(), "pass");
    assert_eq!(CheckStatus::Warning.as_str(), "warning");
    assert_eq!(CheckStatus::Fail.as_str(), "fail");
}

#[test]
fn stable_issue_codes_have_exact_external_names() {
    // Given
    let codes = [
        (IssueCode::InputIo, "input.io"),
        (IssueCode::InputYamlParse, "input.yaml_parse"),
        (IssueCode::InputUnknownField, "input.unknown_field"),
        (IssueCode::InputInvalidValue, "input.invalid_value"),
        (IssueCode::MappingUnsupported, "mapping.unsupported"),
        (
            IssueCode::MappingTargetUnreachable,
            "mapping.target_unreachable",
        ),
        (IssueCode::MappingNonBijective, "mapping.non_bijective"),
        (IssueCode::MappingNonNatural, "mapping.non_natural"),
        (IssueCode::ScenarioInvalid, "scenario.invalid"),
        (IssueCode::ScenarioCaseNotFound, "scenario.case_not_found"),
        (
            IssueCode::ScenarioNoCaseSelected,
            "scenario.no_case_selected",
        ),
        (IssueCode::AddressInvalid, "address.invalid"),
        (IssueCode::AddressOutOfRange, "address.out_of_range"),
        (IssueCode::OutputExists, "output.exists"),
        (IssueCode::OutputInvalidTarget, "output.invalid_target"),
        (
            IssueCode::OutputAtomicUnsupported,
            "output.atomic_unsupported",
        ),
        (IssueCode::OutputIo, "output.io"),
        (IssueCode::OutputTooLarge, "output.too_large"),
        (IssueCode::AnalysisFailed, "analysis.failed"),
    ];

    // When / Then
    for (code, external) in codes {
        assert_eq!(code.as_str(), external);
    }
}
