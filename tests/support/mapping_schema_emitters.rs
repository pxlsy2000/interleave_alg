
#[test]
fn applies_scalar_type_lexeme_and_name_gates() -> TestResult {
    // Given
    let source = "schema_version: \"1\"\nname: \"bad\\u2028name\"\naddress:\n  width_bits: 01\n  granule_bytes: \"64\"\ntargets:\n  count: true\nmapping:\n  m: { rows: [] }\n  l: { mode: 1 }\n";

    // When
    let error = errors(source)?;

    // Then
    let messages: Vec<_> = error
        .issues()
        .iter()
        .map(|issue| (issue.path().as_str(), issue.message()))
        .collect();
    assert_eq!(
        messages,
        [
            ("schema_version", "expected integer, observed \"1\""),
            (
                "name",
                "expected non-empty string without control or line-separator characters, observed \"bad\\u2028name\""
            ),
            ("address.width_bits", "expected plain integer, observed 1"),
            ("address.granule_bytes", "expected integer, observed \"64\""),
            ("targets.count", "expected integer, observed true"),
            ("mapping.l.mode", "expected string, observed 1"),
        ]
    );
    Ok(())
}

#[test]
fn enforces_cross_family_precedence_and_exact_reasons() -> TestResult {
    // Given
    let non_power = "schema_version: 1\nname: bad\naddress: { width_bits: 4, granule_bytes: 3 }\ntargets: { count: 3 }\nmapping: { m: { rows: [] }, l: { mode: preserve_high } }\n";
    let intrinsic = "schema_version: 1\nname: bad\naddress: { width_bits: 4, granule_bytes: 32 }\ntargets: { count: 2 }\nmapping: { m: { rows: [] }, l: { mode: preserve_high } }\n";
    let cap = "schema_version: 1\nname: bad\naddress: { width_bits: 64, granule_bytes: 9007199254740992 }\ntargets: { count: 1 }\nmapping: { m: { rows: [] }, l: { mode: preserve_high } }\n";

    // When
    let first = errors(non_power)?;
    let second = errors(intrinsic)?;
    let third = errors(cap)?;

    // Then
    assert_eq!(issue_at(&first, 0)?.code(), IssueCode::MappingUnsupported);
    assert_eq!(
        issue_at(&first, 0)?.message(),
        "unsupported address.granule_bytes: granule size is not a power of two"
    );
    assert_eq!(
        issue_at(&first, 1)?.message(),
        "unsupported targets.count: target count is not a power of two"
    );
    assert_eq!(
        issue_at(&second, 0)?.message(),
        "expected integer <= 16, observed 32"
    );
    assert_eq!(
        issue_at(&third, 0)?.message(),
        "unsupported address.granule_bytes: granule size exceeds v1 limit 4503599627370496"
    );
    Ok(())
}

#[test]
fn validates_dimensions_taps_duplicates_and_conditional_l_rows() -> TestResult {
    // Given
    let source = "schema_version: 1\nname: shapes\naddress: { width_bits: 8, granule_bytes: 1 }\ntargets: { count: 4 }\nmapping:\n  m:\n    rows:\n      - [0, 0x0]\n      - [8]\n      - []\n  l:\n    mode: preserve_high\n    rows: [[0]]\n";

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
            (
                "mapping.m.rows",
                "expected sequence length 2, observed sequence"
            ),
            (
                "mapping.m.rows[0]",
                "expected unique values, observed sequence"
            ),
            (
                "mapping.m.rows[1][0]",
                "expected integer in [0,7], observed 8"
            ),
            (
                "mapping.l.rows",
                "expected field absent when mapping.l.mode=\"preserve_high\", observed sequence"
            ),
        ]
    );
    Ok(())
}

#[test]
fn explicit_mode_requires_rows_and_preserves_empty_rows() -> TestResult {
    // Given
    let missing = "schema_version: 1\nname: missing\naddress: { width_bits: 4, granule_bytes: 1 }\ntargets: { count: 2 }\nmapping: { m: { rows: [[0]] }, l: { mode: explicit } }\n";
    let empty_row = "schema_version: 1\nname: empty\naddress: { width_bits: 2, granule_bytes: 1 }\ntargets: { count: 2 }\nmapping: { m: { rows: [[0]] }, l: { mode: explicit, rows: [[]] } }\n";

    // When
    let failure = errors(missing)?;
    let accepted = decode(empty_row)?;

    // Then
    assert_eq!(
        issue_at(&failure, 0)?.message(),
        "expected field present when mapping.l.mode=\"explicit\", observed missing"
    );
    assert!(accepted
        .explicit_local_rows()
        .and_then(<[interleave::mapping::XorRow]>::first)
        .is_some_and(|row| row.taps().is_empty()));
    Ok(())
}
