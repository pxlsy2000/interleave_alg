#[test]
fn query_operand_cap_is_checked_in_process_at_exact_and_plus_one() -> TestResult {
    // Given
    let arguments = std::iter::once("interleave")
        .chain(["map", "--spec", NATURAL_MAPPING_PATH])
        .chain(std::iter::repeat_n("0", MAX_QUERY_ADDRESSES + 1));
    let parsed = Cli::try_parse_from(arguments)?;
    let CliCommand::Map(mut map) = parsed.command else {
        return Err("map command must parse as Map".into());
    };
    let width = AddressWidth::new(20)?;

    // When
    let over_limit = preflight_query_addresses(&map.addresses, width);
    map.addresses.truncate(MAX_QUERY_ADDRESSES);
    let at_limit = preflight_query_addresses(&map.addresses, width);

    // Then
    let Err(issues) = over_limit else {
        return Err("cap+1 query must fail".into());
    };
    assert_eq!(issues.len(), 1);
    let issue = issues
        .first()
        .ok_or("cap+1 query must emit exactly one issue")?;
    assert_eq!(issue.code(), IssueCode::InputInvalidValue);
    assert_eq!(issue.path().as_str(), "");
    assert_eq!(
        issue.message(),
        "expected at most 1000000 query addresses, observed 1000001"
    );
    let Ok(accepted) = at_limit else {
        return Err("exact query cap must pass".into());
    };
    assert_eq!(accepted.len(), MAX_QUERY_ADDRESSES);
    Ok(())
}

#[test]
fn negative_hex_reaches_address_validation_after_the_standard_terminator() -> TestResult {
    // Given
    let arguments = [
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        "--format",
        "json",
        "--",
        "-0x1",
    ];

    // When
    let output = interleave(&arguments)?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/errors/0/code")?, "address.invalid");
    assert_eq!(json_at(&envelope, "/errors/0/path")?, "addresses[0]");
    Ok(())
}

#[test]
fn all_bad_operands_are_reported_in_source_order_after_mapping_validation() -> TestResult {
    // Given
    let arguments = [
        "map",
        "--spec",
        NATURAL_MAPPING_PATH,
        "_",
        "0",
        "0x100000",
        "1__0",
        "--format",
        "json",
    ];

    // When
    let output = interleave(&arguments)?;

    // Then
    assert_eq!(output.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_at(&envelope, "/errors/0/path")?, "addresses[0]");
    assert_eq!(json_at(&envelope, "/errors/1/path")?, "addresses[2]");
    assert_eq!(json_at(&envelope, "/errors/2/path")?, "addresses[3]");
    assert_eq!(
        json_at(&envelope, "/errors/1/code")?,
        "address.out_of_range"
    );
    assert!(json_at(&envelope, "/result")?.is_null());
    Ok(())
}
