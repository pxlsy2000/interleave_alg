use thiserror::Error;

use crate::{
    input::scalar::Address,
    issue::{Issue, IssueCode, IssuePath},
};

use super::{
    model::{LocalAddressRows, MappingModel, XorRow},
    validate::validate_mapping,
    validation_model::{MappingClassification, MappingValidation},
};

/// A Mapping that has passed the mathematical validity gates.
#[derive(Debug)]
pub struct AddressMapper<'mapping> {
    mapping: &'mapping MappingModel,
    validation: MappingValidation,
}

impl<'mapping> AddressMapper<'mapping> {
    /// Validates a Mapping and constructs an address mapper only for a valid classification.
    pub fn try_new(mapping: &'mapping MappingModel) -> Result<Self, InvalidMappingError> {
        let validation = validate_mapping(mapping);
        match validation.classification() {
            MappingClassification::ValidNatural | MappingClassification::ValidNonNatural => {
                Ok(Self {
                    mapping,
                    validation,
                })
            }
            MappingClassification::InvalidTargetUnreachable
            | MappingClassification::InvalidNonBijective => Err(InvalidMappingError {
                classification: validation.classification(),
            }),
        }
    }

    /// Returns the completed validation, including a possible non-natural warning.
    pub const fn validation(&self) -> &MappingValidation {
        &self.validation
    }

    /// Maps one byte address after checking the configured exclusive bound.
    pub fn map_address(&self, address: Address) -> Result<MappedAddress, AddressMappingError> {
        let raw = self.preflight(address, 0)?;
        self.map_in_range(address, raw)
    }

    /// Maps a source-ordered address slice without sorting or deduplicating it.
    pub fn map_addresses(
        &self,
        addresses: &[Address],
    ) -> Result<Vec<MappedAddress>, AddressMappingError> {
        for (index, address) in addresses.iter().copied().enumerate() {
            self.preflight(address, index)?;
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(addresses.len())
            .map_err(|_| AddressMappingError::AnalysisFailed)?;
        for address in addresses.iter().copied() {
            rows.push(self.map_in_range(address, address.get())?);
        }
        Ok(rows)
    }

    fn preflight(&self, address: Address, index: usize) -> Result<u128, AddressMappingError> {
        address
            .checked_for_width(self.mapping.address_width())
            .map_err(|_| AddressMappingError::OutOfRange {
                index,
                address,
                width_bits: self.mapping.address_width().get(),
            })
    }

    fn map_in_range(
        &self,
        input_address: Address,
        raw: u128,
    ) -> Result<MappedAddress, AddressMappingError> {
        let granule = u128::from(self.mapping.granule_bytes().get());
        let byte_offset_u128 = raw % granule;
        let line_address_u128 = raw / granule;
        let line_address =
            u64::try_from(line_address_u128).map_err(|_| AddressMappingError::AnalysisFailed)?;
        let target = evaluate_rows(self.mapping.target_rows(), line_address)?;
        let local_line_address = match &self.mapping.local_rows {
            LocalAddressRows::PreserveHigh => line_address >> self.mapping.target_bits(),
            LocalAddressRows::Explicit(rows) => evaluate_rows(rows, line_address)?,
        };
        let local_byte_u128 = u128::from(local_line_address)
            .checked_mul(granule)
            .and_then(|base| base.checked_add(byte_offset_u128))
            .ok_or(AddressMappingError::AnalysisFailed)?;

        Ok(MappedAddress {
            input_address,
            line_address,
            byte_offset: u64::try_from(byte_offset_u128)
                .map_err(|_| AddressMappingError::AnalysisFailed)?,
            target: u16::try_from(target).map_err(|_| AddressMappingError::AnalysisFailed)?,
            local_line_address,
            local_byte_address: u64::try_from(local_byte_u128)
                .map_err(|_| AddressMappingError::AnalysisFailed)?,
        })
    }
}

/// A mathematical classification that cannot be used for address Mapping.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("Mapping classification {classification:?} is invalid")]
pub struct InvalidMappingError {
    classification: MappingClassification,
}

impl InvalidMappingError {
    /// Returns the invalid mathematical classification.
    pub const fn classification(&self) -> MappingClassification {
        self.classification
    }
}

/// The exact result of mapping one input byte address.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MappedAddress {
    input_address: Address,
    line_address: u64,
    byte_offset: u64,
    target: u16,
    local_line_address: u64,
    local_byte_address: u64,
}

impl MappedAddress {
    /// Returns the original byte address.
    pub const fn input_address(&self) -> Address {
        self.input_address
    }

    /// Returns the granule line address.
    pub const fn line_address(&self) -> u64 {
        self.line_address
    }

    /// Returns the preserved byte offset within the granule.
    pub const fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    /// Returns the selected Target ID.
    pub const fn target(&self) -> u16 {
        self.target
    }

    /// Returns the local line address within the selected Target.
    pub const fn local_line_address(&self) -> u64 {
        self.local_line_address
    }

    /// Returns the local byte address within the selected Target.
    pub const fn local_byte_address(&self) -> u64 {
        self.local_byte_address
    }

    /// Returns the canonical input byte address.
    pub fn input_address_canonical(&self) -> String {
        self.input_address.canonical()
    }

    /// Returns the canonical line address.
    pub fn line_address_canonical(&self) -> String {
        canonical_hex(self.line_address)
    }

    /// Returns the canonical byte offset.
    pub fn byte_offset_canonical(&self) -> String {
        canonical_hex(self.byte_offset)
    }

    /// Returns the canonical local line address.
    pub fn local_line_address_canonical(&self) -> String {
        canonical_hex(self.local_line_address)
    }

    /// Returns the canonical local byte address.
    pub fn local_byte_address_canonical(&self) -> String {
        canonical_hex(self.local_byte_address)
    }
}

/// A range or unexpected below-limit address Mapping failure.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum AddressMappingError {
    /// The input byte address is outside the Mapping width.
    #[error("address {address} at index {index} is outside the {width_bits}-bit range")]
    OutOfRange {
        /// Zero-based position in the caller's address sequence.
        index: usize,
        /// Rejected exact address.
        address: Address,
        /// Configured Mapping width.
        width_bits: u8,
    },
    /// A validated, below-limit computation or allocation failed unexpectedly.
    #[error("validated address Mapping failed unexpectedly")]
    AnalysisFailed,
}

impl AddressMappingError {
    /// Returns the source address position when the failure is range-related.
    pub const fn address_index(&self) -> Option<usize> {
        match self {
            Self::OutOfRange { index, .. } => Some(*index),
            Self::AnalysisFailed => None,
        }
    }

    /// Converts the failure into its stable command diagnostic.
    pub fn issue(&self) -> Issue {
        match self {
            Self::OutOfRange {
                index,
                address,
                width_bits,
            } => Issue::new(
                IssueCode::AddressOutOfRange,
                IssuePath::root().field("addresses").index(*index),
                format!("address {address} is outside the {width_bits}-bit range"),
            ),
            Self::AnalysisFailed => Issue::new(
                IssueCode::AnalysisFailed,
                IssuePath::root(),
                "analysis could not be completed",
            ),
        }
    }
}

fn evaluate_rows(rows: &[XorRow], line_address: u64) -> Result<u64, AddressMappingError> {
    let mut output = 0_u64;
    for (output_bit, row) in rows.iter().enumerate() {
        let mut parity = false;
        for tap in row.taps() {
            let mask = 1_u64
                .checked_shl(u32::from(tap.get()))
                .ok_or(AddressMappingError::AnalysisFailed)?;
            parity ^= line_address & mask != 0;
        }
        if parity {
            let bit = u32::try_from(output_bit).map_err(|_| AddressMappingError::AnalysisFailed)?;
            output |= 1_u64
                .checked_shl(bit)
                .ok_or(AddressMappingError::AnalysisFailed)?;
        }
    }
    Ok(output)
}

fn canonical_hex(value: u64) -> String {
    format!("0x{value:x}")
}
