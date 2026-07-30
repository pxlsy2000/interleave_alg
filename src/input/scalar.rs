//! Exact v1 integer, address, range, and ratio primitives.

use std::fmt;

use thiserror::Error;

use super::limits::MAX_ADDRESS_WIDTH_BITS;
use lexeme::{AccumulateError, accumulate, address_digits, generic_digits};

mod lexeme;
mod magnitude;

const DECIMAL_SCALE: u128 = 1_000_000;
const U64_EXCLUSIVE_BOUND: u128 = 18_446_744_073_709_551_616;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AddressMagnitudeError {
    InvalidLexeme,
    AnalysisFailed,
}

pub(crate) struct AddressMagnitude {
    canonical: String,
    value: Option<Address>,
}

/// A generic integer accepted from a plain YAML scalar.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GenericInteger(u128);

impl GenericInteger {
    /// Parses the exact v1 plain-integer grammar.
    pub fn parse(lexeme: &str) -> Result<Self, GenericIntegerError> {
        let (digits, radix) = generic_digits(lexeme)?;
        accumulate(digits, radix, false)
            .map(Self)
            .map_err(|error| match error {
                AccumulateError::Invalid => GenericIntegerError::InvalidLexeme,
                AccumulateError::Overflow => GenericIntegerError::Overflow,
            })
    }

    /// Returns the mathematical value.
    pub const fn get(self) -> u128 {
        self.0
    }

    /// Narrows the value to `u64` only when it is representable.
    pub fn checked_u64(self) -> Result<u64, GenericIntegerError> {
        u64::try_from(self.0).map_err(|_| GenericIntegerError::OutOfRange)
    }
}

/// A byte address accepted from YAML or the command line.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Address(u128);

impl Address {
    pub(crate) const fn from_u128(value: u128) -> Self {
        Self(value)
    }

    /// Parses the exact v1 address grammar, including between-digit underscores.
    pub fn parse(lexeme: &str) -> Result<Self, AddressError> {
        let (digits, radix) = address_digits(lexeme)?;
        accumulate(digits, radix, true)
            .map(Self)
            .map_err(|error| match error {
                AccumulateError::Invalid => AddressError::InvalidLexeme,
                AccumulateError::Overflow => AddressError::Overflow,
            })
    }

    /// Returns the mathematical value.
    pub const fn get(self) -> u128 {
        self.0
    }

    /// Returns canonical lowercase hexadecimal with a `0x` prefix.
    pub fn canonical(self) -> String {
        format!("0x{:x}", self.0)
    }

    /// Returns the mathematical address after proving it lies inside the width.
    pub const fn checked_for_width(self, width: AddressWidth) -> Result<u128, AddressError> {
        if self.0 >= width.exclusive_bound() {
            Err(AddressError::OutOfRange {
                width_bits: width.get(),
            })
        } else {
            Ok(self.0)
        }
    }

    /// Narrows an address after proving it lies inside the requested width.
    pub fn checked_u64(self, width: AddressWidth) -> Result<u64, AddressError> {
        self.checked_for_width(width)
            .and_then(|value| u64::try_from(value).map_err(|_| AddressError::Overflow))
    }
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:x}", self.0)
    }
}

/// A validated address width in the inclusive range 1 through 64.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AddressWidth(u8);

impl AddressWidth {
    /// Constructs a validated address width.
    pub const fn new(width_bits: u8) -> Result<Self, AddressError> {
        if width_bits == 0 || width_bits > MAX_ADDRESS_WIDTH_BITS {
            Err(AddressError::InvalidWidth { width_bits })
        } else {
            Ok(Self(width_bits))
        }
    }

    /// Returns the validated width.
    pub const fn get(self) -> u8 {
        self.0
    }

    /// Returns the exclusive mathematical address bound `2^A`.
    pub const fn exclusive_bound(self) -> u128 {
        if self.0 == MAX_ADDRESS_WIDTH_BITS {
            U64_EXCLUSIVE_BOUND
        } else {
            1_u128 << self.0
        }
    }
}

/// An exact, unreduced non-negative ratio with a positive denominator.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Ratio {
    numerator: u128,
    denominator: u128,
}

impl Ratio {
    /// Constructs a ratio, rejecting a zero denominator.
    pub const fn new(numerator: u128, denominator: u128) -> Result<Self, RatioError> {
        if denominator == 0 {
            Err(RatioError::ZeroDenominator)
        } else {
            Ok(Self {
                numerator,
                denominator,
            })
        }
    }

    /// Returns the unreduced numerator.
    pub const fn numerator(self) -> u128 {
        self.numerator
    }

    /// Returns the unreduced denominator.
    pub const fn denominator(self) -> u128 {
        self.denominator
    }

    /// Formats six decimal places using checked integer round-half-up.
    pub fn decimal_six(self) -> Result<String, RatioError> {
        let scaled = self
            .numerator
            .checked_mul(DECIMAL_SCALE)
            .and_then(|value| value.checked_add(self.denominator / 2))
            .ok_or(RatioError::Overflow)?
            / self.denominator;
        let whole = scaled / DECIMAL_SCALE;
        let fractional = scaled % DECIMAL_SCALE;
        Ok(format!("{whole}.{fractional:06}"))
    }
}

/// Returns whether a mathematical integer is a power of two.
pub const fn is_power_of_two(value: u128) -> bool {
    value.is_power_of_two()
}

/// Returns the exact base-two logarithm for powers of two.
pub const fn exact_log2(value: u128) -> Option<u32> {
    if value.is_power_of_two() {
        Some(value.trailing_zeros())
    } else {
        None
    }
}

/// A generic-integer parse or narrowing failure.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum GenericIntegerError {
    /// The text does not match the exact generic-integer grammar.
    #[error("invalid generic integer lexeme")]
    InvalidLexeme,
    /// The mathematical value exceeds `u128`.
    #[error("generic integer exceeds u128")]
    Overflow,
    /// The mathematical value cannot be represented by `u64`.
    #[error("generic integer exceeds u64")]
    OutOfRange,
}

/// An address parse, width, or range failure.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum AddressError {
    /// The text does not match the exact address grammar.
    #[error("invalid address lexeme")]
    InvalidLexeme,
    /// The mathematical value exceeds `u128`.
    #[error("address exceeds u128")]
    Overflow,
    /// The requested address width is outside 1 through 64.
    #[error("address width {width_bits} is outside 1 through 64")]
    InvalidWidth {
        /// The rejected width.
        width_bits: u8,
    },
    /// The address lies outside the validated address width.
    #[error("address is outside the {width_bits}-bit range")]
    OutOfRange {
        /// The validated width.
        width_bits: u8,
    },
}

/// An exact-ratio construction or formatting failure.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum RatioError {
    /// The denominator was zero.
    #[error("ratio denominator must be positive")]
    ZeroDenominator,
    /// Checked scaling or round-half-up addition overflowed.
    #[error("ratio formatting overflowed")]
    Overflow,
}

#[cfg(test)]
#[path = "scalar_tests.rs"]
mod tests;
