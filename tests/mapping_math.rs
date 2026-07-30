//! GF(2) Mapping validation contract tests.

use interleave::{
    input::load_yaml_bytes,
    issue::{CheckStatus, IssueCode},
    mapping::{
        MappingCheckObservation, MappingClassification, MappingModel, decode_mapping,
        validate_mapping,
    },
};

type TestResult<T = ()> = Result<T, String>;

fn decode(source: &str) -> TestResult<MappingModel> {
    let document = load_yaml_bytes(source.as_bytes()).map_err(|error| error.to_string())?;
    decode_mapping(&document).map_err(|error| {
        error
            .issues()
            .iter()
            .map(|issue| {
                format!(
                    "{}|{}|{}",
                    issue.code().as_str(),
                    issue.path().as_str(),
                    issue.message()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn validate(source: &str) -> TestResult<interleave::mapping::MappingValidation> {
    decode(source).map(|model| validate_mapping(&model))
}

include!("support/mapping_math_core.rs");
include!("support/mapping_math_boundaries.rs");
include!("support/mapping_math_golden.rs");
include!("support/mapping_math_oracle.rs");
