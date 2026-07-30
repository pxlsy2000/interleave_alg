use interleave::report::render_json;
use serde_json::{Value, json};

use super::{
    TestResult,
    fixture::{MappingFixture, validation_report},
};

#[test]
fn valid_natural_validation_matches_golden_shape() -> TestResult {
    // Given
    let report = validation_report(MappingFixture::Natural)?;

    // When
    let actual: Value = serde_json::from_slice(&render_json(&report)?)?;

    // Then
    assert_eq!(actual, validation_golden(GoldenValidation::Natural));
    Ok(())
}

#[test]
fn valid_non_natural_validation_matches_golden_shape() -> TestResult {
    // Given
    let report = validation_report(MappingFixture::NonNatural)?;

    // When
    let actual: Value = serde_json::from_slice(&render_json(&report)?)?;

    // Then
    assert_eq!(actual, validation_golden(GoldenValidation::NonNatural));
    Ok(())
}

#[test]
fn invalid_target_validation_retains_every_check() -> TestResult {
    // Given
    let report = validation_report(MappingFixture::TargetUnreachable)?;

    // When
    let actual: Value = serde_json::from_slice(&render_json(&report)?)?;

    // Then
    assert_eq!(
        actual,
        validation_golden(GoldenValidation::TargetUnreachable)
    );
    Ok(())
}

#[test]
fn invalid_non_bijective_validation_retains_every_check() -> TestResult {
    // Given
    let report = validation_report(MappingFixture::NonBijective)?;

    // When
    let actual: Value = serde_json::from_slice(&render_json(&report)?)?;

    // Then
    assert_eq!(actual, validation_golden(GoldenValidation::NonBijective));
    Ok(())
}

#[derive(Clone, Copy)]
enum GoldenValidation {
    Natural,
    NonNatural,
    TargetUnreachable,
    NonBijective,
}

fn validation_golden(kind: GoldenValidation) -> Value {
    let facts = golden_facts(kind);
    json!({
        "schema_version": 1,
        "command": "validate",
        "status": facts.status,
        "warnings": facts.warnings,
        "errors": facts.errors,
        "result": {
            "mapping_name": "report-map",
            "input": "mapping.yaml",
            "derived": {
                "address_width_bits": 8,
                "granule_bytes": 1,
                "offset_bits": 0,
                "line_bits": 8,
                "target_count": 4,
                "target_bits": 2,
                "local_address_bits": 6
            },
            "checks": [
                {
                    "id": "target_reachable",
                    "status": facts.target_status,
                    "observed": {"rank_m": facts.rank_m},
                    "expected": {"rank_m": 2},
                    "message": facts.target_message
                },
                {
                    "id": "bijective",
                    "status": facts.bijective_status,
                    "observed": {"rank_f": facts.rank_f},
                    "expected": {"rank_f": 8},
                    "message": facts.bijective_message
                },
                {
                    "id": "natural_local_address",
                    "status": facts.natural_status,
                    "observed": {
                        "rank_m_low": facts.rank_m_low,
                        "l_matches_preserve_high": facts.natural_matches
                    },
                    "expected": {"rank_m_low": 2, "l_matches_preserve_high": true},
                    "message": facts.natural_message
                }
            ],
            "classification": facts.classification
        }
    })
}

struct GoldenFacts {
    status: &'static str,
    warnings: Value,
    errors: Value,
    classification: &'static str,
    target_status: &'static str,
    bijective_status: &'static str,
    natural_status: &'static str,
    rank_m: u8,
    rank_f: u8,
    rank_m_low: u8,
    natural_matches: bool,
    target_message: &'static str,
    bijective_message: &'static str,
    natural_message: &'static str,
}

fn golden_facts(kind: GoldenValidation) -> GoldenFacts {
    match kind {
        GoldenValidation::Natural => GoldenFacts {
            status: "pass",
            warnings: json!([]),
            errors: json!([]),
            classification: "valid_natural",
            target_status: "pass",
            bijective_status: "pass",
            natural_status: "pass",
            rank_m: 2,
            rank_f: 8,
            rank_m_low: 2,
            natural_matches: true,
            target_message: "all targets are reachable",
            bijective_message: "mapping is bijective",
            natural_message: "local address is naturally ordered",
        },
        GoldenValidation::NonNatural => GoldenFacts {
            status: "warning",
            warnings: json!([{
                "code": "mapping.non_natural",
                "path": "mapping.l.rows",
                "message": "rank(Mp)=2; L != [0 I]"
            }]),
            errors: json!([]),
            classification: "valid_non_natural",
            target_status: "pass",
            bijective_status: "pass",
            natural_status: "warning",
            rank_m: 2,
            rank_f: 8,
            rank_m_low: 2,
            natural_matches: false,
            target_message: "all targets are reachable",
            bijective_message: "mapping is bijective",
            natural_message: "rank(Mp)=2; L != [0 I]",
        },
        GoldenValidation::TargetUnreachable => GoldenFacts {
            status: "fail",
            warnings: json!([]),
            errors: json!([{
                "code": "mapping.target_unreachable",
                "path": "mapping.m.rows",
                "message": "rank(M)=1, expected 2"
            }]),
            classification: "invalid_target_unreachable",
            target_status: "fail",
            bijective_status: "fail",
            natural_status: "fail",
            rank_m: 1,
            rank_f: 7,
            rank_m_low: 1,
            natural_matches: true,
            target_message: "rank(M)=1, expected 2",
            bijective_message: "rank(F)=7, expected 8",
            natural_message: "rank(Mp)=1, expected 2",
        },
        GoldenValidation::NonBijective => GoldenFacts {
            status: "fail",
            warnings: json!([]),
            errors: json!([{
                "code": "mapping.non_bijective",
                "path": "mapping.l.rows",
                "message": "rank(F)=7, expected 8"
            }]),
            classification: "invalid_non_bijective",
            target_status: "pass",
            bijective_status: "fail",
            natural_status: "fail",
            rank_m: 2,
            rank_f: 7,
            rank_m_low: 2,
            natural_matches: false,
            target_message: "all targets are reachable",
            bijective_message: "rank(F)=7, expected 8",
            natural_message: "rank(Mp)=2; L != [0 I]",
        },
    }
}
