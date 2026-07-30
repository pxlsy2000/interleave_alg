use interleave::{issue::IssueCode, metrics::MetricsError};

#[test]
fn invalid_window_inputs_stop_before_a_metrics_descriptor_exists() -> MetricsTestResult {
    // Given
    let mapping = metrics_mapping(4)?;
    let sources = [
        "schema_version: 1\ndefaults: { accesses: 4, window_sizes: [0] }\n\
         cases:\n  - { name: bad, kind: stride, base_bytes: 0, stride_bytes: 1 }\n",
        "schema_version: 1\ndefaults: { accesses: 4, window_sizes: [1, 1] }\n\
         cases:\n  - { name: bad, kind: stride, base_bytes: 0, stride_bytes: 1 }\n",
    ];

    for source in sources {
        // When
        let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
        let decoded = decode_scenario(&document);

        // Then
        assert!(decoded.is_err());
    }

    // When
    let source = "schema_version: 1\ndefaults: { accesses: 4, window_sizes: [5] }\n\
                  cases:\n  - { name: bad, kind: stride, base_bytes: 0, stride_bytes: 1 }\n";
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    let scenario = decode_scenario(&document).map_err(|error| error.to_string())?;
    let selected = select_cases(&scenario, &[]).map_err(|error| error.to_string())?;
    let preflight = preflight_scenarios(&mapping, &selected);

    // Then
    assert!(preflight.is_err());
    Ok(())
}

#[test]
fn impossible_target_is_a_typed_failure_not_an_indexing_panic() -> MetricsTestResult {
    // Given
    let mapping = metrics_mapping(4)?;
    let windows = metric_windows(&mapping, 1, "1")?;

    // When
    let error = analyze_targets(&[4], mapping.target_count(), &windows)
        .expect_err("target outside N must fail");

    // Then
    assert_eq!(error, MetricsError::AnalysisFailed);
    assert_eq!(error.issue().code(), IssueCode::AnalysisFailed);
    Ok(())
}
