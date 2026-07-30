#[test]
fn non_natural_warning_is_retained_when_scenario_preflight_fails() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults:
  accesses: 2
  window_sizes: [3]
cases:
  - name: invalid
    kind: stride
    base_bytes: 0
    stride_bytes: 0
";
    let (mapping, scenario) =
        write_fixture_pair(&directory, NON_NATURAL_MAPPING, scenario)?;

    // When
    let result = run_json(&mapping, &scenario, &[])?;

    // Then
    assert_eq!(result.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&result.stdout)?;
    assert_eq!(
        json_at(&envelope, "/warnings/0/code")?,
        "mapping.non_natural"
    );
    assert_eq!(json_at(&envelope, "/errors/0/code")?, "scenario.invalid");
    assert_eq!(json_at(&envelope, "/result")?, &Value::Null);
    Ok(())
}

#[test]
fn non_natural_warning_survives_schema_selection_and_preflight_failures() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_source(&directory, "mapping.yaml", NON_NATURAL_MAPPING)?;
    let scenarios = [
        (
            "schema",
            "schema_version: 1\ndefaults:\n  accesses: 1\n  window_sizes: [1]\n\
             cases:\n  - name: bad\n    kind: stride\n    base_bytes: 0\n    stride_bytes: 0\n\
             \x20   aggregate: true\n",
            "input.unknown_field",
        ),
        (
            "selection",
            "schema_version: 1\ndefaults:\n  accesses: 1\n  window_sizes: [1]\n\
             cases:\n  - name: disabled\n    enabled: false\n    kind: stride\n    base_bytes: 0\n\
             \x20   stride_bytes: 0\n",
            "scenario.no_case_selected",
        ),
        (
            "preflight",
            "schema_version: 1\ndefaults:\n  accesses: 1\n  window_sizes: [2]\n\
             cases:\n  - name: bad\n    kind: stride\n    base_bytes: 0\n    stride_bytes: 0\n",
            "scenario.invalid",
        ),
    ];

    // When / Then
    for (name, body, expected_error) in scenarios {
        let scenario = write_source(&directory, &format!("{name}.yaml"), body)?;
        let output = run_json(&mapping, &scenario, &[])?;
        assert_eq!(output.status.code(), Some(3));
        let envelope: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            json_at(&envelope, "/warnings/0/code")?,
            "mapping.non_natural"
        );
        assert_eq!(
            json_at(&envelope, "/errors/0/code")?,
            expected_error
        );
    }
    Ok(())
}

#[test]
fn text_failure_retains_non_natural_warning_and_exit_three() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults:
  accesses: 1
  window_sizes: [2]
cases:
  - name: invalid
    kind: stride
    base_bytes: 0
    stride_bytes: 0
";
    let (mapping, scenario) =
        write_fixture_pair(&directory, NON_NATURAL_MAPPING, scenario)?;

    // When
    let output = interleave(&[
        "run",
        "--spec",
        &mapping,
        "--scenario",
        &scenario,
    ])?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let text = String::from_utf8(output.stderr)?;
    assert!(text.contains("WARNING [mapping.non_natural]"));
    assert!(text.contains("ERROR [scenario.invalid]"));
    Ok(())
}
