use std::fmt::Write as _;

use super::{
    Address, AddressMagnitude, AddressMagnitudeError, AddressWidth, lexeme::address_digits,
};

const DECIMAL_CHUNK_DIGITS: usize = 9;
const DECIMAL_CHUNK_RADIX: u32 = 1_000_000_000;

impl AddressMagnitude {
    pub(crate) fn parse(lexeme: &str) -> Result<Self, AddressMagnitudeError> {
        let (digits, radix) =
            address_digits(lexeme).map_err(|_| AddressMagnitudeError::InvalidLexeme)?;
        match radix {
            10 => Self::from_decimal(digits),
            16 => Self::from_hex(digits),
            _ => Err(AddressMagnitudeError::AnalysisFailed),
        }
    }

    pub(crate) fn in_width(&self, width: AddressWidth) -> Option<Address> {
        self.value
            .filter(|address| address.get() < width.exclusive_bound())
    }

    pub(crate) fn canonical(&self) -> &str {
        &self.canonical
    }

    fn from_decimal(digits: &[u8]) -> Result<Self, AddressMagnitudeError> {
        let digit_count = digits.iter().filter(|byte| **byte != b'_').count();
        let remainder = digit_count % DECIMAL_CHUNK_DIGITS;
        let mut group_remaining = if remainder == 0 {
            DECIMAL_CHUNK_DIGITS
        } else {
            remainder
        };
        let mut limbs = vec![0_u32];
        let mut chunk = 0_u32;
        for byte in digits.iter().copied().filter(|byte| *byte != b'_') {
            let digit = u32::from(byte - b'0');
            chunk = chunk
                .checked_mul(10)
                .and_then(|value| value.checked_add(digit))
                .ok_or(AddressMagnitudeError::AnalysisFailed)?;
            group_remaining = group_remaining
                .checked_sub(1)
                .ok_or(AddressMagnitudeError::AnalysisFailed)?;
            if group_remaining == 0 {
                multiply_add(&mut limbs, DECIMAL_CHUNK_RADIX, chunk)?;
                group_remaining = DECIMAL_CHUNK_DIGITS;
                chunk = 0;
            }
        }
        Self::from_limbs(&limbs)
    }

    fn from_hex(digits: &[u8]) -> Result<Self, AddressMagnitudeError> {
        let mut normalized = Vec::new();
        normalized
            .try_reserve_exact(digits.len())
            .map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        normalized.extend(
            digits
                .iter()
                .copied()
                .filter(|byte| *byte != b'_')
                .map(|byte| byte.to_ascii_lowercase()),
        );
        let first = normalized
            .iter()
            .position(|byte| *byte != b'0')
            .unwrap_or_else(|| normalized.len().saturating_sub(1));
        let significant = normalized
            .get(first..)
            .ok_or(AddressMagnitudeError::AnalysisFailed)?;
        let mut canonical = String::from("0x");
        canonical
            .try_reserve(significant.len())
            .map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        for byte in significant {
            canonical.push(char::from(*byte));
        }
        let value = if significant.len() <= 16 {
            let mut raw = 0_u64;
            for byte in significant {
                raw = raw
                    .checked_mul(16)
                    .and_then(|partial| hex_digit(*byte).map(|digit| partial + u64::from(digit)))
                    .ok_or(AddressMagnitudeError::AnalysisFailed)?;
            }
            Some(Address::from_u128(u128::from(raw)))
        } else {
            None
        };
        Ok(Self { canonical, value })
    }

    fn from_limbs(limbs: &[u32]) -> Result<Self, AddressMagnitudeError> {
        let Some(high) = limbs.last() else {
            return Err(AddressMagnitudeError::AnalysisFailed);
        };
        let mut canonical = format!("0x{high:x}");
        for limb in limbs.iter().rev().skip(1) {
            write!(canonical, "{limb:08x}").map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        }
        let value = match limbs {
            [low] => Some(Address::from_u128(u128::from(*low))),
            [low, high] => {
                let raw = u64::from(*low) | (u64::from(*high) << 32);
                Some(Address::from_u128(u128::from(raw)))
            }
            [] | [_, _, ..] => None,
        };
        Ok(Self { canonical, value })
    }
}

fn multiply_add(
    limbs: &mut Vec<u32>,
    multiplier: u32,
    addend: u32,
) -> Result<(), AddressMagnitudeError> {
    let mut carry = u64::from(addend);
    for limb in limbs.iter_mut() {
        let total = u64::from(*limb)
            .checked_mul(u64::from(multiplier))
            .and_then(|value| value.checked_add(carry))
            .ok_or(AddressMagnitudeError::AnalysisFailed)?;
        *limb = u32::try_from(total & u64::from(u32::MAX))
            .map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        carry = total >> 32;
    }
    if carry != 0 {
        limbs
            .try_reserve(1)
            .map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        limbs.push(u32::try_from(carry).map_err(|_| AddressMagnitudeError::AnalysisFailed)?);
    }
    Ok(())
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
