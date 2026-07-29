#[test]
fn block_scalar_sequence_and_empty_roots_fail_the_mapping_schema_gate() -> TestResult {
    // Given
    let cases = [
        (
            b"hello\n".as_slice(),
            "expected mapping, observed \"hello\"",
        ),
        (
            b"- hello\n".as_slice(),
            "expected mapping, observed sequence",
        ),
        (b"---\n".as_slice(), "expected mapping, observed null"),
    ];

    // When
    let errors: Vec<_> = cases
        .iter()
        .map(|(source, _)| yaml_error(source))
        .collect::<TestResult<_>>()?;

    // Then
    for (error, (_, message)) in errors.iter().zip(cases) {
        assert_eq!(error.issue().code(), IssueCode::InputInvalidValue);
        assert!(error.issue().path().as_str().is_empty());
        assert_eq!(error.issue().message(), message);
    }
    Ok(())
}

#[test]
fn block_mapping_succeeds_while_flow_mapping_remains_yaml_parse_failure() -> TestResult {
    // Given
    let block = b"name: mapping\n";
    let flow = b"{name: mapping}\n";

    // When
    let block_result = load_yaml_bytes(block);
    let flow_error = yaml_error(flow)?;

    // Then
    assert!(block_result.is_ok());
    assert_eq!(flow_error.issue().code(), IssueCode::InputYamlParse);
    assert_eq!(flow_error.issue().message(), "invalid YAML syntax");
    Ok(())
}

#[test]
fn plain_core_floats_and_case_variant_booleans_keep_non_string_types() -> TestResult {
    // Given
    let source = b"decimal: 1.5\nfraction: -.5\ntrailing_dot: 1.\nexponent: 1e3\nsigned_exponent: +1.E-3\nleading_zero: 01\ninf_lower: .inf\ninf_title: -.Inf\ninf_upper: +.INF\nnan_lower: .nan\nnan_title: .NaN\nnan_upper: .NAN\ntrue_title: True\ntrue_upper: TRUE\nfalse_title: False\nfalse_upper: FALSE\n";

    // When
    let document = load_yaml_bytes(source).map_err(|error| error.to_string())?;

    // Then
    let entries = mapping(&document)?;
    for index in 0..12 {
        assert_eq!(scalar_at(entries, index)?.kind(), ScalarKind::Float);
    }
    for index in 12..16 {
        assert_eq!(scalar_at(entries, index)?.kind(), ScalarKind::Boolean);
    }
    assert_eq!(scalar_at(entries, 3)?.value(), "1e3");
    assert_eq!(scalar_at(entries, 5)?.value(), "01");
    assert_eq!(scalar_at(entries, 13)?.value(), "TRUE");
    Ok(())
}

#[test]
fn quoted_core_spellings_remain_strings() -> TestResult {
    // Given
    let source =
        b"float: \"1.5\"\nexponent: '1e3'\ninf: \".INF\"\nnan: '.NaN'\nbool: \"TRUE\"\n";

    // When
    let document = load_yaml_bytes(source).map_err(|error| error.to_string())?;

    // Then
    let entries = mapping(&document)?;
    for index in 0..entries.len() {
        assert_eq!(scalar_at(entries, index)?.kind(), ScalarKind::String);
    }
    Ok(())
}

#[test]
fn every_plain_float_key_is_non_string_and_never_becomes_a_duplicate() -> TestResult {
    // Given
    let spellings = [
        "1.5", "1e3", ".inf", ".Inf", ".INF", "-.inf", "+.INF", ".nan", ".NaN", ".NAN",
    ];

    // When
    let errors: Vec<_> = spellings
        .iter()
        .map(|spelling| yaml_error(format!("{spelling}: first\n{spelling}: second\n").as_bytes()))
        .collect::<TestResult<_>>()?;

    // Then
    assert!(errors.iter().all(|error| {
        error.issue().code() == IssueCode::InputYamlParse
            && error.issue().path().as_str().is_empty()
            && error.issue().message() == "invalid YAML syntax"
            && error.position().is_some_and(|position| position.byte_offset() == 0)
    }));
    Ok(())
}

#[test]
fn case_variant_boolean_keys_are_non_string() -> TestResult {
    // Given
    let spellings = ["True", "TRUE", "False", "FALSE"];

    // When
    let errors: Vec<_> = spellings
        .iter()
        .map(|spelling| yaml_error(format!("{spelling}: value\n").as_bytes()))
        .collect::<TestResult<_>>()?;

    // Then
    assert!(errors.iter().all(|error| {
        error.issue().code() == IssueCode::InputYamlParse
            && error.issue().message() == "invalid YAML syntax"
    }));
    Ok(())
}
