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
    #[serde(skip)]
    pub(super) matrices: Box<MappingMatrixReport>,
}

impl ValidateResult {
    pub(super) fn new(mapping: &MappingModel, input: &str, validation: &MappingValidation) -> Self {
        Self {
            mapping_name: mapping.name().as_str().to_owned(),
            input: input.to_owned(),
            derived: DerivedMapping::new(mapping),
            checks: validation.checks().clone(),
            classification: validation.classification(),
            matrices: Box::new(MappingMatrixReport::new(mapping)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MappingMatrixReport {
    pub(super) line_bits: usize,
    pub(super) target_bits: usize,
    pub(super) target: Vec<Vec<bool>>,
    pub(super) local: Vec<Vec<bool>>,
    pub(super) combined: Vec<Vec<bool>>,
    pub(super) target_low: Vec<Vec<bool>>,
}

impl MappingMatrixReport {
    fn new(mapping: &MappingModel) -> Self {
        let line_bits = usize::from(mapping.line_bits());
        let target_bits = usize::from(mapping.target_bits());
        let target = bit_rows(mapping.target_rows(), line_bits);
        let local = mapping.explicit_local_rows().map_or_else(
            || {
                (0..usize::from(mapping.local_address_bits()))
                    .map(|row| {
                        (0..line_bits)
                            .map(|column| column == target_bits + row)
                            .collect()
                    })
                    .collect()
            },
            |rows| bit_rows(rows, line_bits),
        );
        let combined = target.iter().chain(&local).cloned().collect();
        let target_low = target
            .iter()
            .map(|row| row.iter().take(target_bits).copied().collect())
            .collect();
        Self {
            line_bits,
            target_bits,
            target,
            local,
            combined,
            target_low,
        }
    }
}

fn bit_rows(rows: &[crate::mapping::XorRow], columns: usize) -> Vec<Vec<bool>> {
    rows.iter()
        .map(|row| {
            (0..columns)
                .map(|column| {
                    row.taps()
                        .iter()
                        .any(|tap| usize::from(tap.get()) == column)
                })
                .collect()
        })
        .collect()
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
    #[serde(skip)]
    pub(super) input: Option<String>,
}

impl MapResult {
    pub(super) fn new(
        mapping: &MappingModel,
        validation: &MappingValidation,
        addresses: &[MappedAddress],
        input: Option<&str>,
    ) -> Self {
        Self {
            mapping_name: mapping.name().as_str().to_owned(),
            mapping_classification: validation.classification(),
            addresses: addresses.iter().map(MapAddressRow::from).collect(),
            input: input.map(str::to_owned),
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
