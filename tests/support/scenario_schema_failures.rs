#[test]
fn rejects_empty_shapes_wrong_kind_fields_and_invalid_schedule() -> ScenarioTestResult {
    // Given
    let source = r"schema_version: 1
defaults: { accesses: 0, window_sizes: [] }
cases:
  - { name: empty-sweep, kind: sweep, base_bytes: [], stride_bytes: [] }
  - name: wrong-fields
    kind: stride
    base_bytes: 0
    stride_bytes: 1
    streams: []
  - name: bad-schedule
    kind: multi_stream
    schedule: weighted
    streams: []
    accesses: 7
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
                "defaults.accesses",
                "expected integer in [1,10000000], observed 0"
            ),
            (
                IssueCode::ScenarioInvalid,
                "defaults.window_sizes",
                "expected non-empty sequence, observed sequence"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[0].base_bytes",
                "expected non-empty sequence, observed sequence"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[0].stride_bytes",
                "expected non-empty sequence, observed sequence"
            ),
            (
                IssueCode::InputUnknownField,
                "cases[1][\"streams\"]",
                "unknown field \"streams\""
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[2].schedule",
                "expected one of [\"round_robin\"], observed \"weighted\""
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[2].streams",
                "expected non-empty sequence, observed sequence"
            ),
            (
                IssueCode::ScenarioInvalid,
                "cases[2].accesses",
                "expected field absent when cases[2].kind=\"multi_stream\", observed 7"
            ),
        ]
    );
    Ok(())
}

#[test]
fn combined_failures_follow_declaration_and_source_order_without_fallback() -> ScenarioTestResult {
    // Given
    let source = r"schema_version: 1
defaults: { accesses: 1, window_sizes: [1] }
cases:
  - name: sweep
    kind: sweep
    base_bytes: [64, 0x40]
    stride_bytes: [1]
  - name: mixed
    kind: multi_stream
    capacity: 2
    schedule: weighted
    streams:
      - { name: bad, base_bytes: 0, stride_bytes: 1, accesses: 1, aggregate: true }
";

    // When
    let error = scenario_errors(source)?;

    // Then
    let got = error
        .issues()
        .iter()
        .map(|issue| (issue.code().as_str(), issue.path().as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        got,
        [
            ("scenario.invalid", "cases[0].base_bytes"),
            ("input.unknown_field", "cases[1][\"capacity\"]"),
            ("scenario.invalid", "cases[1].schedule"),
            (
                "input.unknown_field",
                "cases[1].streams[0][\"aggregate\"]"
            ),
        ]
    );
    let mut sorted = error.issues().to_vec();
    interleave::issue::sort_issues(&mut sorted);
    let sorted_paths = sorted
        .iter()
        .map(|issue| issue.path().as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        sorted_paths,
        [
            "cases[0].base_bytes",
            "cases[1][\"capacity\"]",
            "cases[1].schedule",
            "cases[1].streams[0][\"aggregate\"]",
        ]
    );
    Ok(())
}
