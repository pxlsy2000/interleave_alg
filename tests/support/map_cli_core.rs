#[test]
fn many_addresses_preserve_order_duplicates_offsets_and_canonical_forms() -> TestResult {
    // Given
    let arguments = [
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        "0",
        "1_024",
        "0xAB_CD",
        "0x40",
        "0x40",
        "0x41",
    ];

    // When
    let text = interleave(&arguments)?;
    let json = interleave(&[
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        "0",
        "1_024",
        "0xAB_CD",
        "0x40",
        "0x40",
        "0x41",
        "--format",
        "json",
    ])?;

    // Then
    assert_eq!(text.status.code(), Some(0));
    assert!(text.stderr.is_empty());
    assert_eq!(
        std::str::from_utf8(&text.stdout)?,
        include_str!("../golden/map_cli_success.txt")
    );
    assert_eq!(json.status.code(), Some(0));
    assert!(json.stderr.is_empty());
    assert_eq!(
        std::str::from_utf8(&json.stdout)?,
        include_str!("../golden/map_cli_success.json")
    );
    Ok(())
}

#[test]
fn one_address_is_a_complete_report() -> TestResult {
    // Given
    let arguments = [
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        "0x1234",
        "--format",
        "json",
    ];

    // When
    let output = interleave(&arguments)?;

    // Then
    assert_eq!(output.status.code(), Some(0));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/command")?, "map");
    assert_eq!(
        json_at(&envelope, "/result/addresses/0/input_address")?,
        "0x1234"
    );
    assert_eq!(json_at(&envelope, "/result/addresses/0/byte_offset")?, "0x34");
    assert_eq!(
        json_at(&envelope, "/result/addresses/0/local_byte_address")?,
        "0x4b4"
    );
    Ok(())
}

#[test]
fn non_natural_mapping_retains_its_warning() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_mapping(&directory, "non-natural.yaml", NON_NATURAL_MAPPING)?;

    // When
    let output = interleave(&[
        "map", "--spec", &mapping, "0x80", "--format", "json",
    ])?;

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
        json_at(&envelope, "/result/mapping_classification")?,
        "valid_non_natural"
    );
    Ok(())
}
