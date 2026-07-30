#[test]
fn enforces_scalar_type_lexeme_and_range_ladders() -> ScenarioTestResult {
    // Given
    let source = r#"schema_version: 1
defaults:
  accesses: "1"
  window_sizes: [01, 0, 10000001]
cases:
  - name: scalars
    kind: stride
    base_bytes: "64"
    stride_bytes: _1
    accesses: 01
"#;

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
            ("defaults.accesses", "expected integer, observed \"1\""),
            (
                "defaults.window_sizes[0]",
                "expected plain integer, observed 1"
            ),
            (
                "defaults.window_sizes[1]",
                "expected integer in [1,10000000], observed 0"
            ),
            (
                "defaults.window_sizes[2]",
                "expected integer in [1,10000000], observed 10000001"
            ),
            (
                "cases[0].base_bytes",
                "expected plain address, observed \"64\""
            ),
            (
                "cases[0].stride_bytes",
                "expected plain address, observed \"_1\""
            ),
            (
                "cases[0].accesses",
                "expected plain integer, observed 1"
            ),
        ]
    );
    Ok(())
}

#[test]
fn enforces_case_and_stream_name_contracts_and_stream_uniqueness() -> ScenarioTestResult {
    // Given
    let source = r"schema_version: 1
defaults: { accesses: 1, window_sizes: [1] }
cases:
  - { name: _bad, kind: stride, base_bytes: 0, stride_bytes: 0 }
  - name: streams
    kind: multi_stream
    schedule: round_robin
    streams:
      - { name: bad/name, base_bytes: 0, stride_bytes: 0, accesses: 1 }
      - { name: same, base_bytes: 0, stride_bytes: 0, accesses: 1 }
      - { name: same, base_bytes: 0, stride_bytes: 0, accesses: 2 }
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
                "cases[0].name",
                "expected string matching \"[A-Za-z0-9][A-Za-z0-9._-]*\", observed \"_bad\""
            ),
            (
                "cases[1].streams[0].name",
                "expected string matching \"[A-Za-z0-9][A-Za-z0-9._-]*\", observed \"bad/name\""
            ),
            (
                "cases[1].streams",
                "expected unique values, observed sequence"
            ),
        ]
    );
    Ok(())
}

#[test]
fn emits_recursive_present_issues_before_canonical_missing_fields() -> ScenarioTestResult {
    // Given
    let source = r"extra: 1
defaults: []
cases:
  - 1
  - kind: multi_stream
    streams:
      - {}
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
                IssueCode::InputUnknownField,
                "[\"extra\"]",
                "unknown field \"extra\""
            ),
            (
                IssueCode::ScenarioInvalid,
                "defaults",
                "expected mapping, observed sequence"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[0]",
                "expected mapping, observed 1"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[1].streams[0].name",
                "expected required field, observed missing"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[1].streams[0].base_bytes",
                "expected required field, observed missing"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[1].streams[0].stride_bytes",
                "expected required field, observed missing"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[1].streams[0].accesses",
                "expected required field, observed missing"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[1].name",
                "expected required field, observed missing"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[1].schedule",
                "expected required field, observed missing"
            ),
            (
                IssueCode::ScenarioInvalid,
                "schema_version",
                "expected required field, observed missing"
            ),
        ]
    );
    Ok(())
}

#[test]
fn wrong_typed_kind_validates_only_common_fields_and_unknown_keys() -> ScenarioTestResult {
    // Given
    let source = r"schema_version: 1
defaults: { accesses: 1, window_sizes: [1] }
cases:
  - name: common
    enabled: 0
    kind: 7
    window_sizes: {}
    base_bytes: []
    streams: nope
    bad.key: true
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
            (IssueCode::ScenarioInvalid, "cases[0].enabled"),
            (IssueCode::ScenarioInvalid, "cases[0].kind"),
            (IssueCode::ScenarioInvalid, "cases[0].window_sizes"),
            (IssueCode::InputUnknownField, "cases[0][\"bad.key\"]"),
        ]
    );
    Ok(())
}
