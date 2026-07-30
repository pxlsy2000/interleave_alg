#[test]
fn emits_exact_natural_check_array() -> TestResult {
    // Given
    let source = mapping_source(3, 1, &[1], None);

    // When
    let result = validate(&source)?;
    let checks = serde_json::to_value(result.checks()).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(
        checks,
        serde_json::json!([
            {
                "id": "target_reachable",
                "status": "pass",
                "observed": {"rank_m": 1},
                "expected": {"rank_m": 1},
                "message": "all targets are reachable"
            },
            {
                "id": "bijective",
                "status": "pass",
                "observed": {"rank_f": 3},
                "expected": {"rank_f": 3},
                "message": "mapping is bijective"
            },
            {
                "id": "natural_local_address",
                "status": "pass",
                "observed": {
                    "rank_m_low": 1,
                    "l_matches_preserve_high": true
                },
                "expected": {
                    "rank_m_low": 1,
                    "l_matches_preserve_high": true
                },
                "message": "local address is naturally ordered"
            }
        ])
    );
    assert_eq!(
        result.classification(),
        MappingClassification::ValidNatural
    );
    assert!(result.issues().is_empty());
    Ok(())
}

#[test]
fn classifies_l_only_non_natural_with_explicit_path() -> TestResult {
    // Given
    let source = mapping_source(3, 1, &[1], Some(&[3, 4]));

    // When
    let result = validate(&source)?;

    // Then
    assert_eq!(
        result.classification(),
        MappingClassification::ValidNonNatural
    );
    let natural = result
        .checks()
        .last()
        .ok_or_else(|| "missing natural check".to_owned())?;
    assert_eq!(natural.status(), CheckStatus::Warning);
    assert_eq!(natural.message(), "rank(Mp)=1; L != [0 I]");
    assert_eq!(
        natural.observed(),
        &MappingCheckObservation::NaturalLocalAddress {
            rank_m_low: 1,
            l_matches_preserve_high: false,
        }
    );
    assert_issue(
        &result,
        IssueCode::MappingNonNatural,
        "mapping.l.rows",
        "rank(Mp)=1; L != [0 I]",
    )
}

#[test]
fn classifies_both_non_natural_causes_with_mapping_path() -> TestResult {
    // Given
    let source = mapping_source(3, 1, &[2], Some(&[1, 4]));

    // When
    let result = validate(&source)?;

    // Then
    assert_eq!(
        result.classification(),
        MappingClassification::ValidNonNatural
    );
    let natural = result
        .checks()
        .last()
        .ok_or_else(|| "missing natural check".to_owned())?;
    assert_eq!(natural.status(), CheckStatus::Warning);
    assert_eq!(
        natural.message(),
        "rank(Mp)=0, expected 1; L != [0 I]"
    );
    assert_issue(
        &result,
        IssueCode::MappingNonNatural,
        "mapping",
        "rank(Mp)=0, expected 1; L != [0 I]",
    )
}

#[test]
fn retains_all_checks_but_only_unreachable_primary_error() -> TestResult {
    // Given
    let source = mapping_source(3, 1, &[0], None);

    // When
    let result = validate(&source)?;

    // Then
    assert_eq!(
        result.classification(),
        MappingClassification::InvalidTargetUnreachable
    );
    assert_eq!(
        result
            .checks()
            .iter()
            .map(interleave::mapping::MappingCheck::status)
            .collect::<Vec<_>>(),
        [CheckStatus::Fail, CheckStatus::Fail, CheckStatus::Fail]
    );
    assert_issue(
        &result,
        IssueCode::MappingTargetUnreachable,
        "mapping.m.rows",
        "rank(M)=0, expected 1",
    )
}

#[test]
fn target_reachable_precedes_non_bijective() -> TestResult {
    // Given
    let source = mapping_source(3, 2, &[1, 1], Some(&[1]));

    // When
    let result = validate(&source)?;

    // Then
    assert_eq!(
        result.classification(),
        MappingClassification::InvalidTargetUnreachable
    );
    assert_issue(
        &result,
        IssueCode::MappingTargetUnreachable,
        "mapping.m.rows",
        "rank(M)=1, expected 2",
    )
}

#[test]
fn classifies_reachable_rank_deficient_explicit_mapping() -> TestResult {
    // Given
    let source = mapping_source(3, 1, &[1], Some(&[1, 4]));

    // When
    let result = validate(&source)?;

    // Then
    assert_eq!(
        result.classification(),
        MappingClassification::InvalidNonBijective
    );
    assert_issue(
        &result,
        IssueCode::MappingNonBijective,
        "mapping.l.rows",
        "rank(F)=2, expected 3",
    )
}

fn assert_issue(
    result: &interleave::mapping::MappingValidation,
    code: IssueCode,
    path: &str,
    message: &str,
) -> TestResult {
    let issue = result
        .issues()
        .first()
        .ok_or_else(|| "missing mathematical issue".to_owned())?;
    assert_eq!(result.issues().len(), 1);
    assert_eq!(issue.code(), code);
    assert_eq!(issue.path().as_str(), path);
    assert_eq!(issue.message(), message);
    Ok(())
}
