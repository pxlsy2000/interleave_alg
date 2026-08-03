#[test]
fn release_surface_executes_q_one_with_unbounded_unused_stride() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults: { accesses: 1, window_sizes: [1] }
cases:
  - name: once
    kind: stride
    base_bytes: 0
    stride_bytes: 0x100000000000000000000000000000000
";
    let (mapping, scenario) = write_fixture_pair(&directory, NATURAL_MAPPING, scenario)?;

    // When
    let output = run_json(&mapping, &scenario, &[])?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/status")?, "pass");
    assert_eq!(json_at(&envelope, "/result/cases/0/accesses")?, 1);
    assert_eq!(json_at(&envelope, "/result/cases/0/targets/0/count")?, 1);
    Ok(())
}

#[test]
fn release_surface_reports_exact_unbounded_last_address_without_partial_result() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults: { accesses: 2, window_sizes: [1] }
cases:
  - name: twice
    kind: stride
    base_bytes: 1
    stride_bytes: 0x100000000000000000000000000000000
";
    let (mapping, scenario) = write_fixture_pair(&directory, NATURAL_MAPPING, scenario)?;

    // When
    let output = run_json(&mapping, &scenario, &[])?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/errors/0/code")?, "address.out_of_range");
    assert_eq!(json_at(&envelope, "/errors/0/path")?, "cases[0]");
    assert_eq!(
        json_at(&envelope, "/errors/0/message")?,
        "address 0x100000000000000000000000000000001 is outside the 20-bit range"
    );
    assert_eq!(json_at(&envelope, "/result")?, &Value::Null);
    Ok(())
}

#[test]
fn decoded_address_scalar_limit_precedes_scenario_range_preflight() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let exact_address = format!("0x1{}", "0".repeat(125));
    let oversized_address = format!("0x1{}", "0".repeat(126));
    let exact_scenario = format!(
        "schema_version: 1\ndefaults: {{ accesses: 1, window_sizes: [1] }}\ncases:\n\
         \x20 - {{ name: exact, kind: stride, base_bytes: {exact_address}, stride_bytes: 0 }}\n"
    );
    let oversized_scenario = format!(
        "schema_version: 1\ndefaults: {{ accesses: 1, window_sizes: [1] }}\ncases:\n\
         \x20 - {{ name: oversized, kind: stride, base_bytes: {oversized_address}, stride_bytes: 0 }}\n"
    );
    let mapping = write_source(&directory, "mapping.yaml", NATURAL_MAPPING)?;
    let exact = write_source(&directory, "exact.yaml", &exact_scenario)?;
    let oversized = write_source(&directory, "oversized.yaml", &oversized_scenario)?;

    // When
    let exact_output = run_json(&mapping, &exact, &[])?;
    let oversized_output = run_json(&mapping, &oversized, &[])?;

    // Then
    let exact_json: Value = serde_json::from_slice(&exact_output.stdout)?;
    let oversized_json: Value = serde_json::from_slice(&oversized_output.stdout)?;
    assert_eq!(exact_output.status.code(), Some(3));
    assert_eq!(json_at(&exact_json, "/errors/0/code")?, "address.out_of_range");
    assert_eq!(json_at(&exact_json, "/errors/0/path")?, "cases[0]");
    assert_eq!(oversized_output.status.code(), Some(3));
    assert_eq!(
        json_at(&oversized_json, "/errors/0/code")?,
        "input.invalid_value"
    );
    assert_eq!(json_at(&oversized_json, "/errors/0/path")?, "");
    Ok(())
}
