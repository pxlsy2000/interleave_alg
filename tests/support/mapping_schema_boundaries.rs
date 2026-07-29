
#[test]
fn enforces_intrinsic_relations_before_support_caps() -> TestResult {
    // Given
    let granule = "schema_version: 1\nname: intrinsic-g\naddress: { width_bits: 52, granule_bytes: 9007199254740992 }\ntargets: { count: 1 }\nmapping: { m: { rows: [] }, l: { mode: preserve_high } }\n";
    let targets = "schema_version: 1\nname: intrinsic-n\naddress: { width_bits: 16, granule_bytes: 1 }\ntargets: { count: 131072 }\nmapping: { m: { rows: [] }, l: { mode: preserve_high } }\n";

    // When
    let granule_error = errors(granule)?;
    let target_error = errors(targets)?;

    // Then
    assert_eq!(
        issue_at(&granule_error, 0)?.message(),
        "expected integer <= 4503599627370496, observed 9007199254740992"
    );
    assert_eq!(
        issue_at(&target_error, 0)?.message(),
        "expected integer <= 65536, observed 131072"
    );
    Ok(())
}

#[test]
fn rejects_target_cap_only_after_intrinsic_relation_passes() -> TestResult {
    // Given
    let source = "schema_version: 1\nname: cap-n\naddress: { width_bits: 64, granule_bytes: 1 }\ntargets: { count: 131072 }\nmapping: { m: { rows: [] }, l: { mode: preserve_high } }\n";

    // When
    let error = errors(source)?;

    // Then
    assert_eq!(
        issue_at(&error, 0)?.code(),
        IssueCode::MappingUnsupported
    );
    assert_eq!(
        issue_at(&error, 0)?.message(),
        "unsupported targets.count: target count exceeds v1 limit 65536"
    );
    assert_eq!(error.issues().len(), 1);
    Ok(())
}

#[test]
fn reports_wrong_containers_without_descendant_cascades() -> TestResult {
    // Given
    let source = "schema_version: 1\nname: containers\naddress: []\ntargets: null\nmapping:\n  m: text\n  l: []\n";

    // When
    let error = errors(source)?;

    // Then
    let got: Vec<_> = error
        .issues()
        .iter()
        .map(|issue| (issue.path().as_str(), issue.message()))
        .collect();
    assert_eq!(
        got,
        [
            ("address", "expected mapping, observed sequence"),
            ("targets", "expected mapping, observed null"),
            ("mapping.m", "expected mapping, observed \"text\""),
            ("mapping.l", "expected mapping, observed sequence"),
        ]
    );
    Ok(())
}

#[test]
fn applies_first_constraint_to_rows_and_taps() -> TestResult {
    // Given
    let source = "schema_version: 1\nname: tap-gates\naddress: { width_bits: 8, granule_bytes: 1 }\ntargets: { count: 2 }\nmapping:\n  m:\n    rows:\n      - [\"0\", -1, 1.5, 8]\n  l:\n    mode: explicit\n    rows: [text, [], [], [], [], [], []]\n";

    // When
    let error = errors(source)?;

    // Then
    let got: Vec<_> = error
        .issues()
        .iter()
        .map(|issue| (issue.path().as_str(), issue.message()))
        .collect();
    assert_eq!(
        got,
        [
            ("mapping.m.rows[0][0]", "expected integer, observed \"0\""),
            ("mapping.m.rows[0][1]", "expected plain integer, observed -1"),
            ("mapping.m.rows[0][2]", "expected integer, observed \"1.5\""),
            ("mapping.m.rows[0][3]", "expected integer in [0,7], observed 8"),
            ("mapping.l.rows[0]", "expected sequence, observed \"text\""),
        ]
    );
    Ok(())
}

#[test]
fn reports_explicit_row_count_and_keeps_row_source_order() -> TestResult {
    // Given
    let source = "schema_version: 1\nname: l-count\naddress: { width_bits: 4, granule_bytes: 1 }\ntargets: { count: 2 }\nmapping:\n  m: { rows: [[0]] }\n  l: { mode: explicit, rows: [[1], [2]] }\n";

    // When
    let error = errors(source)?;

    // Then
    assert_eq!(error.issues().len(), 1);
    assert_eq!(issue_at(&error, 0)?.path().as_str(), "mapping.l.rows");
    assert_eq!(
        issue_at(&error, 0)?.message(),
        "expected sequence length 3, observed sequence"
    );
    Ok(())
}

#[test]
fn accepts_core_strings_and_exact_128_byte_name() -> TestResult {
    // Given
    let name = "a".repeat(128);
    let source = format!(
        "schema_version: 1\nname: {name}\naddress: {{ width_bits: 1, granule_bytes: 1 }}\ntargets: {{ count: 1 }}\nmapping: {{ m: {{ rows: [] }}, l: {{ mode: preserve_high }} }}\n"
    );
    let on = "schema_version: 1\nname: on\naddress: { width_bits: 1, granule_bytes: 1 }\ntargets: { count: 1 }\nmapping: { m: { rows: [] }, l: { mode: preserve_high } }\n";

    // When
    let exact = decode(&source)?;
    let core_string = decode(on)?;

    // Then
    assert_eq!(exact.name().as_str().len(), 128);
    assert_eq!(core_string.name().as_str(), "on");
    Ok(())
}

#[test]
fn decoded_scalar_cap_precedes_mapping_schema() -> TestResult {
    // Given
    let overlong = "a".repeat(129);
    let source = format!("name: {overlong}\nunknown: 1\n");

    // When
    let error = load_yaml_bytes(source.as_bytes())
        .err()
        .ok_or_else(|| "overlong scalar was accepted".to_owned())?;

    // Then
    assert_eq!(error.issue().code(), IssueCode::InputInvalidValue);
    assert!(error.issue().path().as_str().is_empty());
    assert_eq!(
        error.issue().message(),
        "expected UTF-8 byte length <= 128, observed 129"
    );
    Ok(())
}

#[test]
fn handles_integer_lexemes_larger_than_u128_without_crashing() -> TestResult {
    // Given
    let power = format!("0x1{}", "0".repeat(40));
    let non_power = format!("{}3", "9".repeat(40));
    let power_source = format!("schema_version: 1\nname: huge\naddress: {{ width_bits: 64, granule_bytes: {power} }}\ntargets: {{ count: 1 }}\nmapping: {{ m: {{ rows: [] }}, l: {{ mode: preserve_high }} }}\n");
    let non_power_source = format!("schema_version: 1\nname: huge\naddress: {{ width_bits: 64, granule_bytes: {non_power} }}\ntargets: {{ count: 1 }}\nmapping: {{ m: {{ rows: [] }}, l: {{ mode: preserve_high }} }}\n");

    // When
    let first = errors(&power_source)?;
    let second = errors(&non_power_source)?;

    // Then
    assert!(issue_at(&first, 0)?
        .message()
        .starts_with("expected integer <= 18446744073709551616, observed "));
    assert_eq!(
        issue_at(&second, 0)?.message(),
        "unsupported address.granule_bytes: granule size is not a power of two"
    );
    Ok(())
}
