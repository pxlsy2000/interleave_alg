use crate::input::scalar::AddressWidth;

/// A validated Mapping name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MappingName(String);

impl MappingName {
    pub(super) const fn new(value: String) -> Self {
        Self(value)
    }

    /// Returns the decoded name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated power-of-two granule size.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GranuleBytes(u64);

impl GranuleBytes {
    pub(super) const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the granule size in bytes.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A validated power-of-two Target count.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TargetCount(u32);

impl TargetCount {
    pub(super) const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the number of Targets.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A validated input-line bit position used by an XOR row.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct XorTap(u8);

impl XorTap {
    pub(super) const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Returns the zero-based input-line bit position.
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// One source-ordered XOR row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XorRow(Vec<XorTap>);

impl XorRow {
    pub(super) const fn new(taps: Vec<XorTap>) -> Self {
        Self(taps)
    }

    /// Returns taps in declaration order.
    pub fn taps(&self) -> &[XorTap] {
        &self.0
    }
}

/// The selected local-address matrix representation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LocalAddressMode {
    /// Derive the natural `[0 I]` matrix.
    PreserveHigh,
    /// Use the explicitly declared rows.
    Explicit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum LocalAddressRows {
    PreserveHigh,
    Explicit(Vec<XorRow>),
}

/// A structurally valid Mapping with checked derived dimensions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingModel {
    pub(super) name: MappingName,
    pub(super) address_width: AddressWidth,
    pub(super) granule_bytes: GranuleBytes,
    pub(super) target_count: TargetCount,
    pub(super) offset_bits: u8,
    pub(super) line_bits: u8,
    pub(super) target_bits: u8,
    pub(super) local_address_bits: u8,
    pub(super) target_rows: Vec<XorRow>,
    pub(super) local_rows: LocalAddressRows,
}

impl MappingModel {
    /// Returns the Mapping name.
    pub const fn name(&self) -> &MappingName {
        &self.name
    }

    /// Returns the total byte-address width.
    pub const fn address_width(&self) -> AddressWidth {
        self.address_width
    }

    /// Returns the access granule size.
    pub const fn granule_bytes(&self) -> GranuleBytes {
        self.granule_bytes
    }

    /// Returns the Target count.
    pub const fn target_count(&self) -> TargetCount {
        self.target_count
    }

    /// Returns `g`.
    pub const fn offset_bits(&self) -> u8 {
        self.offset_bits
    }

    /// Returns `n`.
    pub const fn line_bits(&self) -> u8 {
        self.line_bits
    }

    /// Returns `r`.
    pub const fn target_bits(&self) -> u8 {
        self.target_bits
    }

    /// Returns `s`.
    pub const fn local_address_bits(&self) -> u8 {
        self.local_address_bits
    }

    /// Returns Target XOR rows in output-bit order.
    pub fn target_rows(&self) -> &[XorRow] {
        &self.target_rows
    }

    /// Returns the selected local-address representation.
    pub const fn local_address_mode(&self) -> LocalAddressMode {
        match self.local_rows {
            LocalAddressRows::PreserveHigh => LocalAddressMode::PreserveHigh,
            LocalAddressRows::Explicit(_) => LocalAddressMode::Explicit,
        }
    }

    /// Returns explicit local-address rows, if declared.
    pub fn explicit_local_rows(&self) -> Option<&[XorRow]> {
        match &self.local_rows {
            LocalAddressRows::PreserveHigh => None,
            LocalAddressRows::Explicit(rows) => Some(rows),
        }
    }
}
