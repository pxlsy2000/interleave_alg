#[test]
fn all_documented_kinds_expand_to_fourteen_ordered_reports() -> TestResult {
    // Given
    let scenario = "assets/templates/scenario.yaml";
    let mapping = "assets/templates/mapping.yaml";

    // When
    let output = run_json(mapping, scenario, &[])?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    let cases = json_at(&envelope, "/result/cases")?
        .as_array()
        .ok_or("cases must be an array")?;
    assert_eq!(cases.len(), 14);
    assert_eq!(json_at(&envelope, "/result/cases/0/case_id")?, "sequential");
    assert_eq!(
        json_at(&envelope, "/result/cases/1/case_id")?,
        "stride-and-phase-sweep[base=0x0,stride=0x40]"
    );
    assert_eq!(
        json_at(&envelope, "/result/cases/12/case_id")?,
        "stride-and-phase-sweep[base=0xc0,stride=0x100]"
    );
    assert_eq!(json_at(&envelope, "/result/cases/13/case_id")?, "two-master");
    assert_eq!(
        json_at(&envelope, "/result/cases/0/targets")?
            .as_array()
            .map(Vec::len),
        Some(4)
    );
    assert_eq!(
        json_at(&envelope, "/result/cases/0/windows/0/size")?,
        4
    );
    Ok(())
}

#[test]
fn explicit_repeated_selection_ignores_enabled_and_uses_declaration_order() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults:
  accesses: 4
  window_sizes: [2]
cases:
  - name: disabled-first
    enabled: false
    kind: stride
    base_bytes: 0
    stride_bytes: 64
  - name: enabled-second
    enabled: true
    kind: stride
    base_bytes: 64
    stride_bytes: 64
";
    let (mapping, scenario) = write_fixture_pair(&directory, NATURAL_MAPPING, scenario)?;

    // When
    let output = run_json(
        &mapping,
        &scenario,
        &["enabled-second", "disabled-first", "enabled-second"],
    )?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        json_at(&envelope, "/result/cases/0/source_case")?,
        "disabled-first"
    );
    assert_eq!(
        json_at(&envelope, "/result/cases/1/source_case")?,
        "enabled-second"
    );
    assert_eq!(
        json_at(&envelope, "/result/cases")?
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    Ok(())
}

#[test]
fn run_text_and_json_are_deterministic_and_complete() -> TestResult {
    // Given
    let arguments = [
        "run",
        "--spec",
        "assets/templates/mapping.yaml",
        "--scenario",
        "assets/templates/scenario.yaml",
        "--case",
        "sequential",
    ];

    // When
    let first = interleave(&arguments)?;
    let second = interleave(&arguments)?;
    let json = run_json(
        "assets/templates/mapping.yaml",
        "assets/templates/scenario.yaml",
        &["sequential"],
    )?;

    // Then
    assert_eq!(first.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
    assert!(first.stderr.is_empty());
    assert_eq!(
        std::str::from_utf8(&first.stdout)?,
        include_str!("../golden/run_cli_sequential.txt")
    );
    assert_eq!(
        std::str::from_utf8(&json.stdout)?,
        include_str!("../golden/run_cli_sequential.json")
    );
    let envelope: Value = serde_json::from_slice(&json.stdout)?;
    assert_eq!(json_at(&envelope, "/command")?, "run");
    assert_eq!(json_at(&envelope, "/status")?, "pass");
    assert_eq!(json_at(&envelope, "/result/cases/0/accesses")?, 4096);
    Ok(())
}

#[test]
fn non_natural_mapping_warning_propagates_to_run() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let scenario = r"schema_version: 1
defaults:
  accesses: 2
  window_sizes: [1]
cases:
  - name: one
    kind: stride
    base_bytes: 0
    stride_bytes: 64
";
    let (mapping, scenario) = write_fixture_pair(&directory, NON_NATURAL_MAPPING, scenario)?;

    // When
    let output = run_json(&mapping, &scenario, &[])?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/status")?, "warning");
    assert_eq!(
        json_at(&envelope, "/warnings/0/code")?,
        "mapping.non_natural"
    );
    assert_eq!(
        json_at(&envelope, "/result/mapping_classification")?,
        "valid_non_natural"
    );
    Ok(())
}

#[test]
fn either_input_can_be_the_sole_standard_input_source() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_source(&directory, "mapping.yaml", NATURAL_MAPPING)?;
    let scenario = write_source(
        &directory,
        "scenario.yaml",
        interleave::templates::scenario(),
    )?;

    // When
    let mapping_stdin = interleave_stdin(
        &[
            "run",
            "--spec",
            "-",
            "--scenario",
            &scenario,
            "--case",
            "sequential",
            "--format",
            "json",
        ],
        NATURAL_MAPPING.as_bytes(),
    )?;
    let scenario_stdin = interleave_stdin(
        &[
            "run",
            "--spec",
            &mapping,
            "--scenario",
            "-",
            "--case",
            "sequential",
            "--format",
            "json",
        ],
        interleave::templates::scenario().as_bytes(),
    )?;

    // Then
    for output in [mapping_stdin, scenario_stdin] {
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let envelope: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(json_at(&envelope, "/result/cases/0/case_id")?, "sequential");
    }
    Ok(())
}
