#[test]
fn no_address_operand_is_a_usage_error() -> TestResult {
    // Given / When
    let output = interleave(&["map", "--spec", NATURAL_MAPPING_PATH])?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    Ok(())
}

#[test]
fn force_without_a_path_output_is_rejected_before_input_read() -> TestResult {
    // Given
    let arguments = [
        "map",
        "--spec",
        "/definitely/missing/interleave-mapping.yaml",
        "0",
        "--force",
    ];

    // When
    let output = interleave(&arguments)?;

    // Then
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("--force option requires a path-valued --output"));
    assert!(!stderr.contains("[input.io]"));
    Ok(())
}

#[test]
fn existing_output_is_refused_then_force_replaced() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let destination = directory.path().join("map.json");
    fs::write(&destination, b"original\n")?;
    let output_path = destination.to_string_lossy().into_owned();

    // When
    let refused = interleave(&[
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        "0",
        "--format",
        "json",
        "--output",
        &output_path,
    ])?;
    let forced = interleave(&[
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        "0",
        "--format",
        "json",
        "--output",
        &output_path,
        "--force",
    ])?;

    // Then
    assert_eq!(refused.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("[output.exists]"));
    assert_eq!(forced.status.code(), Some(0));
    assert!(forced.stdout.is_empty());
    assert!(forced.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&fs::read(destination)?)?;
    assert_eq!(json_at(&envelope, "/command")?, "map");
    Ok(())
}

#[test]
fn query_failure_leaves_existing_output_byte_for_byte_untouched() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let destination = directory.path().join("map.txt");
    fs::write(&destination, b"original\n")?;
    let output_path = destination.to_string_lossy().into_owned();

    // When
    let output = interleave(&[
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        "0",
        "_",
        "--output",
        &output_path,
        "--force",
    ])?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("[address.invalid]"));
    assert_eq!(fs::read(destination)?, b"original\n");
    Ok(())
}
