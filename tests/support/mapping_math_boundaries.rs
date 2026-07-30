#[test]
fn handles_zero_dimensions_and_explicit_preserve_high_equivalence() -> TestResult {
    // Given
    let zero = "schema_version: 1\nname: zero\naddress: { width_bits: 1, granule_bytes: 2 }\ntargets: { count: 1 }\nmapping:\n  m: { rows: [] }\n  l: { mode: explicit, rows: [] }\n";
    let explicit = mapping_source(3, 1, &[1], Some(&[2, 4]));

    // When
    let zero_result = validate(zero)?;
    let explicit_result = validate(&explicit)?;

    // Then
    assert_eq!(
        zero_result.classification(),
        MappingClassification::ValidNatural
    );
    assert_eq!(
        explicit_result.classification(),
        MappingClassification::ValidNatural
    );
    Ok(())
}

#[test]
fn accepts_bit_63_without_shift_overflow() -> TestResult {
    // Given
    let source = mapping_source(64, 1, &[1_u64 << 63], None);

    // When
    let result = validate(&source)?;

    // Then
    assert_eq!(
        result.classification(),
        MappingClassification::InvalidNonBijective
    );
    assert_eq!(
        result
            .checks()
            .first()
            .ok_or_else(|| "missing target check".to_owned())?
            .observed(),
        &MappingCheckObservation::TargetReachable { rank_m: 1 }
    );
    assert_issue(
        &result,
        IssueCode::MappingNonBijective,
        "mapping.m.rows",
        "rank(F)=63, expected 64",
    )
}
