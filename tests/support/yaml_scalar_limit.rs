#[test]
fn decoded_scalar_bound_accepts_exact_plain_and_quoted_key_values() {
    // Given
    let plain = "a".repeat(128);
    let escaped = "\\u0061".repeat(128);
    let sources = [
        format!("{plain}: value\n"),
        format!("key: {plain}\n"),
        format!("\"{escaped}\": value\n"),
        format!("key: \"{escaped}\"\n"),
        format!("key: {}\n", "é".repeat(64)),
    ];

    // When
    let results: Vec<_> = sources
        .iter()
        .map(|source| load_yaml_bytes(source.as_bytes()))
        .collect();

    // Then
    assert!(results.iter().all(Result::is_ok));
}

#[test]
fn decoded_scalar_bound_rejects_plain_quoted_and_multibyte_key_values_at_plus_one() -> TestResult {
    // Given
    let plain = "a".repeat(129);
    let escaped = "\\u0061".repeat(129);
    let sources = [
        (format!("{plain}: value\n"), 129),
        (format!("key: {plain}\n"), 129),
        (format!("\"{escaped}\": value\n"), 129),
        (format!("key: \"{escaped}\"\n"), 129),
        (format!("key: {}\n", "é".repeat(65)), 130),
    ];

    // When
    let errors: Vec<_> = sources
        .iter()
        .map(|(source, _)| yaml_error(source.as_bytes()))
        .collect::<TestResult<_>>()?;

    // Then
    for (error, (_, observed)) in errors.iter().zip(sources) {
        assert_eq!(error.issue().code(), IssueCode::InputInvalidValue);
        assert!(error.issue().path().as_str().is_empty());
        assert_eq!(
            error.issue().message(),
            format!("expected UTF-8 byte length <= 128, observed {observed}")
        );
    }
    Ok(())
}

#[test]
fn first_overlong_decoded_scalar_wins_by_source_byte_position() -> TestResult {
    // Given
    let first = "a".repeat(130);
    let later = "b".repeat(129);
    let source = format!("ok: short\n{first}: value\nlater: {later}\n");

    // When
    let error = yaml_error(source.as_bytes())?;

    // Then
    assert_eq!(error.issue().code(), IssueCode::InputInvalidValue);
    assert_eq!(
        error.issue().message(),
        "expected UTF-8 byte length <= 128, observed 130"
    );
    assert!(error.position().is_some_and(|position| {
        position.byte_offset() == "ok: short\n".len()
    }));
    Ok(())
}

#[test]
fn prohibited_yaml_retains_precedence_over_decoded_scalar_bound() -> TestResult {
    // Given
    let long_string = "a".repeat(129);
    let long_integer = "1".repeat(129);
    let tagged = format!("key: !text {long_string}\n");
    let non_string_key = format!("{long_integer}: value\n");
    let duplicate = format!("{long_string}: first\n{long_string}: second\n");

    // When
    let tagged_error = yaml_error(tagged.as_bytes())?;
    let key_error = yaml_error(non_string_key.as_bytes())?;
    let duplicate_error = yaml_error(duplicate.as_bytes())?;

    // Then
    for error in [&tagged_error, &key_error, &duplicate_error] {
        assert_eq!(error.issue().code(), IssueCode::InputYamlParse);
    }
    assert_eq!(key_error.issue().message(), "invalid YAML syntax");
    assert!(key_error.position().is_some_and(|position| position.byte_offset() == 0));
    assert_eq!(
        duplicate_error.issue().message(),
        format!("duplicate key \"{long_string}\"")
    );
    Ok(())
}
