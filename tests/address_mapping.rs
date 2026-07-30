//! Contract tests for exact byte-address Mapping.

use interleave::{
    input::{load_yaml_bytes, scalar::Address},
    issue::IssueCode,
    mapping::{
        AddressMapper, AddressMappingError, MappedAddress, MappingClassification, MappingModel,
        decode_mapping,
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

fn address(value: &str) -> TestResult<Address> {
    Address::parse(value).map_err(|error| error.to_string())
}

include!("support/address_mapping_fixture.rs");
include!("support/address_mapping_core.rs");
include!("support/address_mapping_boundaries.rs");
include!("support/address_mapping_oracle.rs");
