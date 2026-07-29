use super::{ApplicationError, BoundaryError, DomainError, ExitClass, InputKind};
use crate::issue::IssueCode;

#[test]
fn exit_classes_use_the_fixed_process_codes() {
    // Given / When / Then
    assert_eq!(ExitClass::Success.code(), 0);
    assert_eq!(ExitClass::UsageOrIo.code(), 1);
    assert_eq!(ExitClass::Mapping.code(), 2);
    assert_eq!(ExitClass::ScenarioOrAddress.code(), 3);
    assert_eq!(ExitClass::Analysis.code(), 4);
}

#[test]
fn boundary_errors_always_exit_one_with_typed_issue_codes() {
    // Given
    let cases = [
        (BoundaryError::Usage, None),
        (
            BoundaryError::InputIo {
                kind: InputKind::Mapping,
            },
            Some(IssueCode::InputIo),
        ),
        (BoundaryError::OutputExists, Some(IssueCode::OutputExists)),
        (
            BoundaryError::OutputInvalidTarget,
            Some(IssueCode::OutputInvalidTarget),
        ),
        (
            BoundaryError::OutputAtomicUnsupported,
            Some(IssueCode::OutputAtomicUnsupported),
        ),
        (BoundaryError::OutputIo, Some(IssueCode::OutputIo)),
        (
            BoundaryError::OutputTooLarge,
            Some(IssueCode::OutputTooLarge),
        ),
    ];

    // When / Then
    for (error, issue_code) in cases {
        assert_eq!(error.exit_class(), ExitClass::UsageOrIo);
        assert_eq!(error.issue_code(), issue_code);
    }
}

#[test]
fn domain_errors_map_to_mapping_scenario_and_analysis_classes() {
    // Given
    let cases = [
        (
            DomainError::InvalidInput {
                kind: InputKind::Mapping,
            },
            ExitClass::Mapping,
        ),
        (DomainError::UnsupportedMapping, ExitClass::Mapping),
        (DomainError::InvalidMapping, ExitClass::Mapping),
        (
            DomainError::InvalidInput {
                kind: InputKind::Scenario,
            },
            ExitClass::ScenarioOrAddress,
        ),
        (DomainError::InvalidScenario, ExitClass::ScenarioOrAddress),
        (DomainError::InvalidAddress, ExitClass::ScenarioOrAddress),
        (DomainError::AnalysisFailed, ExitClass::Analysis),
    ];

    // When / Then
    for (error, expected) in cases {
        assert_eq!(error.exit_class(), expected);
        assert_eq!(ApplicationError::from(error).exit_class(), expected);
    }
}
