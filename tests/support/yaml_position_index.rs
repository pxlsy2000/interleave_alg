use std::fmt::Write as _;

#[test]
fn prohibited_properties_after_unicode_bom_and_crlf_keep_exact_positions() -> TestResult {
    // Given
    let bom_tag = "前: ok\n后: !t value\n";
    let mut bom_source = b"\xef\xbb\xbf".to_vec();
    bom_source.extend_from_slice(bom_tag.as_bytes());
    let crlf_anchor = "first: ok\r\n后: &a value\r\n";
    let cases = [
        (
            bom_source,
            3 + "前: ok\n后: ".len(),
            2_usize,
            3_usize,
        ),
        (
            crlf_anchor.as_bytes().to_vec(),
            "first: ok\r\n后: ".len(),
            2,
            3,
        ),
    ];

    // When / Then
    for (source, byte, line, column) in cases {
        let error = yaml_error(&source)?;
        let position = error
            .position()
            .ok_or_else(|| "missing prohibited-property position".to_owned())?;
        assert_eq!(error.issue().code(), IssueCode::InputYamlParse);
        assert_eq!(error.issue().path().as_str(), "");
        assert_eq!(position.byte_offset(), byte);
        assert_eq!(position.line(), line);
        assert_eq!(position.column(), column);
    }
    Ok(())
}

#[test]
fn hundred_thousand_tag_anchor_entries_keep_the_first_stable_issue() -> TestResult {
    // Given
    let mut source = String::new();
    for index in 0..100_000 {
        writeln!(source, "k{index}: !t &a{index} value").map_err(|error| error.to_string())?;
    }

    // When
    let error = yaml_error(source.as_bytes())?;

    // Then
    let position = error
        .position()
        .ok_or_else(|| "missing first prohibited-property position".to_owned())?;
    assert_eq!(error.issue().code(), IssueCode::InputYamlParse);
    assert_eq!(error.issue().path().as_str(), "");
    assert_eq!(error.issue().message(), "invalid YAML syntax");
    assert_eq!(position.byte_offset(), 4);
    assert_eq!(position.line(), 1);
    assert_eq!(position.column(), 4);
    Ok(())
}
