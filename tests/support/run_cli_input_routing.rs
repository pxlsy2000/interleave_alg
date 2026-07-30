#[test]
fn oversized_scenario_uses_exit_three_before_output_preflight() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_source(&directory, "mapping.yaml", NATURAL_MAPPING)?;
    let scenario = directory.path().join("oversized.yaml");
    fs::write(
        &scenario,
        vec![b' '; interleave::input::limits::MAX_RAW_INPUT_BYTES + 1],
    )?;
    let output = directory.path().join("existing.json");
    fs::write(&output, b"prior\n")?;
    let scenario_arg = scenario.to_string_lossy().into_owned();
    let output_arg = output.to_string_lossy().into_owned();

    // When
    let result = interleave(&[
        "run",
        "--spec",
        &mapping,
        "--scenario",
        &scenario_arg,
        "--format",
        "json",
        "--output",
        &output_arg,
    ])?;

    // Then
    assert_eq!(result.status.code(), Some(3));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("input.invalid_value"));
    assert_untouched(&output, b"prior\n")?;
    Ok(())
}

#[test]
fn overlong_decoded_scenario_scalar_is_a_json_business_failure() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_source(&directory, "mapping.yaml", NATURAL_MAPPING)?;
    let name = "a".repeat(129);
    let scenario_body = format!(
        "schema_version: 1\ndefaults:\n  accesses: 1\n  window_sizes: [1]\n\
         cases:\n  - name: {name}\n    kind: stride\n    base_bytes: 0\n    stride_bytes: 0\n"
    );
    let scenario = write_source(&directory, "scalar.yaml", &scenario_body)?;

    // When
    let result = run_json(&mapping, &scenario, &[])?;

    // Then
    assert_eq!(result.status.code(), Some(3));
    assert!(result.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&result.stdout)?;
    assert_eq!(json_at(&envelope, "/errors/0/code")?, "input.invalid_value");
    assert_eq!(json_at(&envelope, "/errors/0/path")?, "");
    assert_eq!(json_at(&envelope, "/result")?, &Value::Null);
    Ok(())
}

#[test]
fn json_preflight_failure_atomically_replaces_forced_report_destination() -> TestResult {
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
    let (mapping, scenario) = write_fixture_pair(&directory, NATURAL_MAPPING, scenario)?;
    let output = directory.path().join("failure.json");
    fs::write(&output, b"prior\n")?;
    let output_arg = output.to_string_lossy().into_owned();

    // When
    let result = interleave(&[
        "run",
        "--spec",
        &mapping,
        "--scenario",
        &scenario,
        "--format",
        "json",
        "--output",
        &output_arg,
        "--force",
    ])?;

    // Then
    assert_eq!(result.status.code(), Some(3));
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&fs::read(&output)?)?;
    assert_eq!(json_at(&envelope, "/errors/0/code")?, "scenario.invalid");
    assert_eq!(json_at(&envelope, "/result")?, &Value::Null);
    Ok(())
}
