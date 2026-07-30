use crate::{
    input::{load_yaml_bytes, scalar::Address},
    issue::IssueCode,
    mapping::{AddressMapper, MappingModel, decode_mapping},
};

use super::{
    AccessCount, ConcreteStimulus, ConcreteTestDescriptor, LinearAccess, StreamAccess, StreamName,
    WindowSize,
    generate::{RoundRobinSchedule, ScenarioGenerationError, generate_target_sequence},
};

type GenerateTestResult<T = ()> = Result<T, String>;

fn test_mapping() -> GenerateTestResult<MappingModel> {
    let document = load_yaml_bytes(
        b"schema_version: 1
name: identity-target
address: { width_bits: 8, granule_bytes: 1 }
targets: { count: 4 }
mapping:
  m: { rows: [[0], [1]] }
  l: { mode: preserve_high }
",
    )
    .map_err(|error| error.to_string())?;
    decode_mapping(&document).map_err(|error| error.to_string())
}

fn linear(base: u128, stride: u128, accesses: u64) -> LinearAccess {
    LinearAccess::new(
        Address::from_u128(base),
        Address::from_u128(stride),
        AccessCount::new(accesses),
    )
}

#[test]
fn ended_streams_are_removed_from_the_active_queue() -> GenerateTestResult {
    // Given
    let streams = vec![
        StreamAccess::new(StreamName::new("A".to_owned()), linear(0, 1, 1)),
        StreamAccess::new(StreamName::new("B".to_owned()), linear(0, 1, 3)),
    ];
    let mut schedule = RoundRobinSchedule::new(&streams).map_err(|error| error.to_string())?;
    assert_eq!(schedule.active_len(), 2);

    // When
    let mut observed = Vec::new();
    while let Some(access) = schedule.next_access().map_err(|error| error.to_string())? {
        observed.push((access.linear, access.index, schedule.active_len()));
    }

    // Then
    assert_eq!(
        observed,
        [
            (linear(0, 1, 1), 0, 1),
            (linear(0, 1, 3), 0, 1),
            (linear(0, 1, 3), 1, 1),
            (linear(0, 1, 3), 2, 0),
        ]
    );
    Ok(())
}

#[test]
fn inconsistent_preflight_token_returns_typed_analysis_failure() -> GenerateTestResult {
    // Given
    let mapping = test_mapping()?;
    let mapper = AddressMapper::try_new(&mapping).map_err(|error| error.to_string())?;
    let descriptor = ConcreteTestDescriptor {
        case_id: "inconsistent".to_owned(),
        source_case: "inconsistent".to_owned(),
        accesses: AccessCount::new(2),
        window_sizes: vec![WindowSize::new(1)],
        stimulus: ConcreteStimulus::Linear(linear(0, 1, 1)),
    };

    // When
    let error = generate_target_sequence(&mapper, &descriptor)
        .expect_err("the deliberately inconsistent token must fail");

    // Then
    assert_eq!(error, ScenarioGenerationError::AnalysisFailed);
    assert_eq!(error.issue().code(), IssueCode::AnalysisFailed);
    assert_eq!(error.issue().path().as_str(), "");
    Ok(())
}

#[test]
fn inconsistent_overflow_token_returns_typed_analysis_failure() -> GenerateTestResult {
    // Given
    let mapping = test_mapping()?;
    let mapper = AddressMapper::try_new(&mapping).map_err(|error| error.to_string())?;
    let descriptor = ConcreteTestDescriptor {
        case_id: "overflow".to_owned(),
        source_case: "overflow".to_owned(),
        accesses: AccessCount::new(2),
        window_sizes: vec![WindowSize::new(1)],
        stimulus: ConcreteStimulus::Linear(linear(u128::MAX, 1, 2)),
    };

    // When
    let error = generate_target_sequence(&mapper, &descriptor)
        .expect_err("the deliberately overflowing token must fail");

    // Then
    assert_eq!(error, ScenarioGenerationError::AnalysisFailed);
    assert_eq!(error.issue().code(), IssueCode::AnalysisFailed);
    Ok(())
}
