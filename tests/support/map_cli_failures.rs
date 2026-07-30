#[test]
fn invalid_mapping_precedes_every_query_failure() -> TestResult {
    // Given
    let directory = TempDir::new()?;
    let mapping = write_mapping(&directory, "broken.yaml", NON_BIJECTIVE_MAPPING)?;

    // When
    let output = interleave(&[
        "map", "--spec", &mapping, "_", "0x100", "--format", "json",
    ])?;

    // Then
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        json_at(&envelope, "/errors/0/code")?,
        "mapping.non_bijective"
    );
    assert!(json_at(&envelope, "/result")?.is_null());
    assert_eq!(
        envelope
            .to_string()
            .matches("address.invalid")
            .count(),
        0
    );
    Ok(())
}

#[test]
fn every_invalid_address_grammar_edge_is_rejected_without_rows() -> TestResult {
    // Given
    let invalid = [
        "_1", "1_", "1__0", "0x_1", "0x1_", "0x1__0", "00", "01", "+1", "-1", "+0",
        "-0", "+0x1", "0X1", "0x", "0xg",
    ];

    // When / Then
    for lexeme in invalid {
        let output = interleave(&[
            "map",
            "--spec",
            NATURAL_MAPPING_PATH,
            lexeme,
            "--format",
            "json",
        ])?;
        assert_eq!(output.status.code(), Some(3), "{lexeme}");
        assert!(output.stderr.is_empty(), "{lexeme}");
        let envelope: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            json_at(&envelope, "/errors/0/code")?,
            "address.invalid",
            "{lexeme}"
        );
        assert_eq!(json_at(&envelope, "/errors/0/path")?, "addresses[0]");
        assert!(json_at(&envelope, "/result")?.is_null());
    }
    Ok(())
}

#[test]
fn exact_address_bound_passes_and_exclusive_bound_fails() -> TestResult {
    // Given
    let maximum = "0xfffff";
    let bound = "0x100000";

    // When
    let accepted = interleave(&[
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        maximum,
        "--format",
        "json",
    ])?;
    let rejected = interleave(&[
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        bound,
        "--format",
        "json",
    ])?;

    // Then
    assert_eq!(accepted.status.code(), Some(0));
    assert_eq!(rejected.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&rejected.stdout)?;
    assert_eq!(
        json_at(&envelope, "/errors/0/code")?,
        "address.out_of_range"
    );
    assert_eq!(json_at(&envelope, "/errors/0/path")?, "addresses[0]");
    assert!(json_at(&envelope, "/result")?.is_null());
    Ok(())
}

#[test]
fn a_late_bad_address_suppresses_every_preceding_valid_row() -> TestResult {
    // Given
    let arguments = [
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        "0",
        "0x40",
        "bad",
    ];

    // When
    let output = interleave(&arguments)?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("[address.invalid] addresses[2]"));
    assert!(!stderr.contains("Address  Line address"));
    Ok(())
}
