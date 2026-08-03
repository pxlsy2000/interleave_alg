use std::fmt::Write as _;

use super::{
    Address, AddressMagnitude, AddressMagnitudeError, AddressWidth, lexeme::address_digits,
};

const DECIMAL_CHUNK_DIGITS: usize = 9;
const DECIMAL_CHUNK_RADIX: u32 = 1_000_000_000;

impl AddressMagnitude {
    /// Parses the exact address grammar without imposing an integer-width cap.
    pub fn parse(lexeme: &str) -> Result<Self, AddressMagnitudeError> {
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

    /// Returns canonical lowercase hexadecimal with a `0x` prefix.
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    pub(crate) fn checked_last(
        &self,
        stride: &Self,
        accesses: u64,
    ) -> Result<Self, AddressMagnitudeError> {
        let multiplier = u32::try_from(accesses.saturating_sub(1))
            .map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        let mut limbs = stride.limbs.clone();
        multiply_add(&mut limbs, multiplier, 0)?;
        add_limbs(&mut limbs, &self.limbs)?;
        Self::from_limbs(limbs)
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
        Self::from_limbs(limbs)
    }

    fn from_hex(digits: &[u8]) -> Result<Self, AddressMagnitudeError> {
        let mut limbs = vec![0_u32];
        for byte in digits.iter().copied().filter(|byte| *byte != b'_') {
            let digit = hex_digit(byte.to_ascii_lowercase())
                .ok_or(AddressMagnitudeError::AnalysisFailed)?;
            multiply_add(&mut limbs, 16, u32::from(digit))?;
        }
        Self::from_limbs(limbs)
    }

    fn from_limbs(mut limbs: Vec<u32>) -> Result<Self, AddressMagnitudeError> {
        while limbs.len() > 1 && limbs.last() == Some(&0) {
            limbs.pop();
        }
        let Some(high) = limbs.last() else {
            return Err(AddressMagnitudeError::AnalysisFailed);
        };
        let mut canonical = format!("0x{high:x}");
        for limb in limbs.iter().rev().skip(1) {
            write!(canonical, "{limb:08x}").map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        }
        let value = match limbs.as_slice() {
            [low] => Some(Address::from_u128(u128::from(*low))),
            [low, high] => {
                let raw = u64::from(*low) | (u64::from(*high) << 32);
                Some(Address::from_u128(u128::from(raw)))
            }
            [] | [_, _, ..] => None,
        };
        Ok(Self {
            limbs,
            canonical,
            value,
        })
    }
}

fn add_limbs(target: &mut Vec<u32>, addend: &[u32]) -> Result<(), AddressMagnitudeError> {
    if target.len() < addend.len() {
        target
            .try_reserve(addend.len() - target.len())
            .map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        target.resize(addend.len(), 0);
    }
    let mut carry = 0_u64;
    for (index, limb) in target.iter_mut().enumerate() {
        let addend_limb = addend.get(index).copied().unwrap_or(0);
        let total = u64::from(*limb)
            .checked_add(u64::from(addend_limb))
            .and_then(|value| value.checked_add(carry))
            .ok_or(AddressMagnitudeError::AnalysisFailed)?;
        *limb = u32::try_from(total & u64::from(u32::MAX))
            .map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        carry = total >> 32;
    }
    if carry != 0 {
        target
            .try_reserve(1)
            .map_err(|_| AddressMagnitudeError::AnalysisFailed)?;
        target.push(u32::try_from(carry).map_err(|_| AddressMagnitudeError::AnalysisFailed)?);
    }
    Ok(())
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
