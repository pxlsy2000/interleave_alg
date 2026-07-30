#[test]
fn both_inputs_cannot_use_standard_input() -> TestResult {
    // Given
    let arguments = ["run", "--spec", "-", "--scenario", "-"];

    // When
    let output = interleave(&arguments)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("standard input"));
    Ok(())
}

#[test]
fn mapping_failure_precedes_malformed_scenario() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_source(&directory, "mapping.yaml", "schema_version: nope\n")?;
    let scenario = write_source(&directory, "scenario.yaml", "not: [valid\n")?;

    // When
    let output = run_json(&mapping, &scenario, &[])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/command")?, "run");
    assert_eq!(json_at(&envelope, "/errors/0/code")?, "input.invalid_value");
    Ok(())
}

#[test]
fn structurally_malformed_unselected_case_still_fails_whole_document() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults:
  accesses: 2
  window_sizes: [1]
cases:
  - name: selected
    kind: stride
    base_bytes: 0
    stride_bytes: 64
  - name: malformed
    enabled: false
    kind: stride
    base_bytes: 0
    stride_bytes: 64
    aggregate: true
";
    let (mapping, scenario) = write_fixture_pair(&directory, NATURAL_MAPPING, scenario)?;

    // When
    let output = run_json(&mapping, &scenario, &["selected"])?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/errors/0/code")?, "input.unknown_field");
    assert_eq!(json_at(&envelope, "/result")?, &Value::Null);
    Ok(())
}

#[test]
fn unselected_semantic_failure_is_ignored_but_selected_inherited_failure_is_not() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults:
  accesses: 2
  window_sizes: [3]
cases:
  - name: valid
    kind: stride
    window_sizes: [1]
    base_bytes: 0
    stride_bytes: 64
  - name: inherited-bad
    enabled: false
    kind: stride
    base_bytes: 0
    stride_bytes: 64
";
    let (mapping, scenario) = write_fixture_pair(&directory, NATURAL_MAPPING, scenario)?;

    // When
    let valid = run_json(&mapping, &scenario, &["valid"])?;
    let invalid = run_json(&mapping, &scenario, &["inherited-bad"])?;

    // Then
    assert_eq!(valid.status.code(), Some(0));
    assert_eq!(invalid.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&invalid.stdout)?;
    assert_eq!(json_at(&envelope, "/errors/0/code")?, "scenario.invalid");
    assert_eq!(json_at(&envelope, "/errors/0/path")?, "defaults.window_sizes[0]");
    Ok(())
}

#[test]
fn generated_address_carry_fails_before_any_case_result_or_file_change() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults:
  accesses: 2
  window_sizes: [1]
cases:
  - name: valid-first
    kind: stride
    base_bytes: 0
    stride_bytes: 64
  - name: carry
    kind: stride
    base_bytes: 0xf_ffc0
    stride_bytes: 64
";
    let (mapping, scenario) = write_fixture_pair(&directory, NATURAL_MAPPING, scenario)?;
    let output = directory.path().join("report.json");
    fs::write(&output, b"prior\n")?;
    let output_arg = output.to_string_lossy().into_owned();

    // When
    let result = interleave(&[
        "run",
        "--spec",
        &mapping,
        "--scenario",
        &scenario,
        "--output",
        &output_arg,
        "--force",
    ])?;

    // Then
    assert_eq!(result.status.code(), Some(3));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("address.out_of_range"));
    assert_untouched(&output, b"prior\n")?;
    Ok(())
}

#[test]
fn missing_and_empty_selection_have_deterministic_failures() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults:
  accesses: 1
  window_sizes: [1]
cases:
  - name: disabled
    enabled: false
    kind: stride
    base_bytes: 0
    stride_bytes: 0
";
    let (mapping, scenario) = write_fixture_pair(&directory, NATURAL_MAPPING, scenario)?;

    // When
    let missing = run_json(&mapping, &scenario, &["absent", "absent"])?;
    let empty = run_json(&mapping, &scenario, &[])?;

    // Then
    assert_eq!(missing.status.code(), Some(3));
    let missing_json: Value = serde_json::from_slice(&missing.stdout)?;
    assert_eq!(
        json_at(&missing_json, "/errors")?.as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        json_at(&missing_json, "/errors/0/code")?,
        "scenario.case_not_found"
    );
    assert_eq!(empty.status.code(), Some(3));
    let empty_json: Value = serde_json::from_slice(&empty.stdout)?;
    assert_eq!(
        json_at(&empty_json, "/errors/0/code")?,
        "scenario.no_case_selected"
    );
    Ok(())
}

#[test]
fn existing_output_refusal_is_atomic() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let (mapping, scenario) =
        write_fixture_pair(&directory, NATURAL_MAPPING, interleave::templates::scenario())?;
    let output = directory.path().join("report.txt");
    fs::write(&output, b"prior\n")?;
    let output_arg = output.to_string_lossy().into_owned();

    // When
    let result = interleave(&[
        "run",
        "--spec",
        &mapping,
        "--scenario",
        &scenario,
        "--output",
        &output_arg,
    ])?;

    // Then
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("output.exists"));
    assert_untouched(&output, b"prior\n")?;
    Ok(())
}
