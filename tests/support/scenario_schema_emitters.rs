#[test]
fn rejects_structural_faults_in_disabled_and_unselected_cases() -> ScenarioTestResult {
    // Given
    let source = r"schema_version: 1
defaults: { accesses: 4, window_sizes: [1] }
cases:
  - { name: good, kind: stride, base_bytes: 0, stride_bytes: 1 }
  - { name: disabled, enabled: false, kind: stride, base_bytes: bad, stride_bytes: 1 }
";

    // When
    let error = scenario_errors(source)?;

    // Then
    let issue = error
        .issues()
        .first()
        .ok_or_else(|| "missing disabled-case issue".to_owned())?;
    assert_eq!(issue.code(), IssueCode::ScenarioInvalid);
    assert_eq!(issue.path().as_str(), "cases[1].base_bytes");
    assert_eq!(
        issue.message(),
        "expected plain address, observed \"bad\""
    );
    Ok(())
}

#[test]
fn invalid_kind_checks_common_fields_and_suppresses_kind_dependent_cascades() -> ScenarioTestResult {
    // Given
    let source = r"schema_version: 1
defaults: { accesses: 1, window_sizes: [1] }
cases:
  - enabled: nope
    kind: mystery
    base_bytes: bad
    schedule: 7
    capacity: 9
";

    // When
    let error = scenario_errors(source)?;

    // Then
    let got = error
        .issues()
        .iter()
        .map(|issue| (issue.code(), issue.path().as_str(), issue.message()))
        .collect::<Vec<_>>();
    assert_eq!(
        got,
        [
            (
                IssueCode::ScenarioInvalid,
                "cases[0].enabled",
                "expected boolean, observed \"nope\""
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[0].kind",
                "expected one of [\"stride\",\"sweep\",\"multi_stream\"], observed \"mystery\""
            ),
            (
                IssueCode::InputUnknownField,
                "cases[0][\"capacity\"]",
                "unknown field \"capacity\""
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[0].name",
                "expected required field, observed missing"
            ),
        ]
    );
    Ok(())
}

#[test]
fn missing_kind_accepts_union_only_for_unknown_detection() -> ScenarioTestResult {
    // Given
    let source = r"schema_version: 1
defaults: { accesses: 1, window_sizes: [1] }
cases:
  - name: no-kind
    base_bytes: bad
    stride_bytes: []
    accesses: nope
    schedule: 7
    streams: nope
    trace: true
";

    // When
    let error = scenario_errors(source)?;

    // Then
    let got = error
        .issues()
        .iter()
        .map(|issue| (issue.code(), issue.path().as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        got,
        [
            (IssueCode::InputUnknownField, "cases[0][\"trace\"]"),
            (IssueCode::ScenarioInvalid, "cases[0].kind"),
        ]
    );
    Ok(())
}

#[test]
fn rejects_duplicate_names_windows_and_normalized_sweep_addresses() -> ScenarioTestResult {
    // Given
    let source = r"schema_version: 1
defaults: { accesses: 1, window_sizes: [1, 1] }
cases:
  - name: repeated
    kind: sweep
    base_bytes: [64, 0x40]
    stride_bytes: [1, 0x1]
  - { name: repeated, kind: stride, base_bytes: 0, stride_bytes: 0 }
";

    // When
    let error = scenario_errors(source)?;

    // Then
    let got = error
        .issues()
        .iter()
        .map(|issue| (issue.path().as_str(), issue.message()))
        .collect::<Vec<_>>();
    assert_eq!(
        got,
        [
            (
                "defaults.window_sizes",
                "expected unique values, observed sequence"
            ),
            (
                "cases[0].base_bytes",
                "expected unique values, observed sequence"
            ),
            (
                "cases[0].stride_bytes",
                "expected unique values, observed sequence"
            ),
            ("cases", "expected unique values, observed sequence"),
        ]
    );
    Ok(())
}
