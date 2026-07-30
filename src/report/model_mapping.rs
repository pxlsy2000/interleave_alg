use serde::Serialize;

use crate::mapping::{MappedAddress, MappingClassification, MappingModel, MappingValidation};

use super::ReportModelError;

/// Mapping dimensions derived from a structurally valid source.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct DerivedMapping {
    pub(super) address_width_bits: u8,
    pub(super) granule_bytes: u64,
    pub(super) offset_bits: u8,
    pub(super) line_bits: u8,
    pub(super) target_count: u32,
    pub(super) target_bits: u8,
    pub(super) local_address_bits: u8,
}

impl DerivedMapping {
    const fn new(mapping: &MappingModel) -> Self {
        Self {
            address_width_bits: mapping.address_width().get(),
            granule_bytes: mapping.granule_bytes().get(),
            offset_bits: mapping.offset_bits(),
            line_bits: mapping.line_bits(),
            target_count: mapping.target_count().get(),
            target_bits: mapping.target_bits(),
            local_address_bits: mapping.local_address_bits(),
        }
    }
}

/// Complete validate-command result, including all mathematical checks.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ValidateResult {
    pub(super) mapping_name: String,
    pub(super) input: String,
    pub(super) derived: DerivedMapping,
    pub(super) checks: [crate::mapping::MappingCheck; 3],
    pub(super) classification: MappingClassification,
}

impl ValidateResult {
    pub(super) fn new(mapping: &MappingModel, input: &str, validation: &MappingValidation) -> Self {
        Self {
            mapping_name: mapping.name().as_str().to_owned(),
            input: input.to_owned(),
            derived: DerivedMapping::new(mapping),
            checks: validation.checks().clone(),
            classification: validation.classification(),
        }
    }
}

/// Canonical JSON representation of one mapped address.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MapAddressRow {
    pub(super) input_address: String,
    pub(super) line_address: String,
    pub(super) byte_offset: String,
    pub(super) target: u16,
    pub(super) local_line_address: String,
    pub(super) local_byte_address: String,
}

impl From<&MappedAddress> for MapAddressRow {
    fn from(address: &MappedAddress) -> Self {
        Self {
            input_address: address.input_address_canonical(),
            line_address: address.line_address_canonical(),
            byte_offset: address.byte_offset_canonical(),
            target: address.target(),
            local_line_address: address.local_line_address_canonical(),
            local_byte_address: address.local_byte_address_canonical(),
        }
    }
}

/// Complete map-command result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MapResult {
    pub(super) mapping_name: String,
    pub(super) mapping_classification: MappingClassification,
    pub(super) addresses: Vec<MapAddressRow>,
}

impl MapResult {
    pub(super) fn new(
        mapping: &MappingModel,
        validation: &MappingValidation,
        addresses: &[MappedAddress],
    ) -> Self {
        Self {
            mapping_name: mapping.name().as_str().to_owned(),
            mapping_classification: validation.classification(),
            addresses: addresses.iter().map(MapAddressRow::from).collect(),
        }
    }
}

pub(super) const fn valid_query_classification(
    classification: MappingClassification,
) -> Result<(), ReportModelError> {
    match classification {
        MappingClassification::ValidNatural | MappingClassification::ValidNonNatural => Ok(()),
        MappingClassification::InvalidTargetUnreachable
        | MappingClassification::InvalidNonBijective => {
            Err(ReportModelError::InvalidMappingClassification)
        }
    }
}
