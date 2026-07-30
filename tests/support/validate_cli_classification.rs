#[test]
fn generated_mapping_template_validates_in_text_json_and_verbose_modes() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = directory.path().join("mapping.yaml");
    let path = mapping.to_string_lossy().into_owned();
    let generated = interleave(&["template", "mapping", "--output", &path])?;
    assert_eq!(generated.status.code(), Some(0));

    // When
    let text = interleave(&["validate", "--spec", &path])?;
    let json = interleave(&["validate", "--spec", &path, "--format", "json"])?;
    let verbose = interleave(&["validate", "--spec", &path, "--verbose"])?;

    // Then
    assert_eq!(text.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&text.stdout).contains("Result: valid_natural"));
    assert!(text.stderr.is_empty());
    assert_eq!(json.status.code(), Some(0));
    let envelope: Value = serde_json::from_slice(&json.stdout)?;
    assert_eq!(json_at(&envelope, "/status")?, "pass");
    assert_eq!(
        json_at(&envelope, "/result/classification")?,
        "valid_natural"
    );
    assert!(json.stderr.is_empty());
    assert_eq!(verbose.status.code(), Some(0));
    let verbose_text = String::from_utf8_lossy(&verbose.stdout);
    for matrix in ["M (", "L (", "F (", "Mp ("] {
        assert!(verbose_text.contains(matrix));
    }
    assert!(verbose.stderr.is_empty());
    Ok(())
}

#[test]
fn non_natural_mapping_is_a_successful_warning() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_mapping(&directory, "non-natural.yaml", NON_NATURAL_MAPPING)?;

    // When
    let output = interleave(&["validate", "--spec", &mapping, "--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/status")?, "warning");
    assert_eq!(
        json_at(&envelope, "/warnings/0/code")?,
        "mapping.non_natural"
    );
    assert_eq!(
        json_at(&envelope, "/result/classification")?,
        "valid_non_natural"
    );
    Ok(())
}

#[test]
fn invalid_mathematics_keeps_a_complete_result_and_exits_two() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_mapping(&directory, "broken.yaml", NON_BIJECTIVE_MAPPING)?;

    // When
    let output = interleave(&["validate", "--spec", &mapping, "--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/status")?, "fail");
    assert_eq!(
        json_at(&envelope, "/errors/0/code")?,
        "mapping.non_bijective"
    );
    assert_eq!(
        json_at(&envelope, "/result/classification")?,
        "invalid_non_bijective"
    );
    Ok(())
}

#[test]
fn target_unreachable_is_the_other_invalid_validation_classification() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_mapping(
        &directory,
        "unreachable.yaml",
        TARGET_UNREACHABLE_MAPPING,
    )?;

    // When
    let output = interleave(&["validate", "--spec", &mapping, "--format", "json"])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        json_at(&envelope, "/errors/0/code")?,
        "mapping.target_unreachable"
    );
    assert_eq!(
        json_at(&envelope, "/result/classification")?,
        "invalid_target_unreachable"
    );
    Ok(())
}
