use super::{AddressError, GenericIntegerError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AccumulateError {
    Invalid,
    Overflow,
}

pub(super) fn generic_digits(lexeme: &str) -> Result<(&[u8], u128), GenericIntegerError> {
    if let Some(hex) = lexeme.strip_prefix("0x") {
        if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(GenericIntegerError::InvalidLexeme);
        }
        Ok((hex.as_bytes(), 16))
    } else {
        let bytes = lexeme.as_bytes();
        let valid = bytes == b"0"
            || bytes
                .first()
                .is_some_and(|byte| matches!(byte, b'1'..=b'9'))
                && bytes.iter().skip(1).all(u8::is_ascii_digit);
        if valid {
            Ok((bytes, 10))
        } else {
            Err(GenericIntegerError::InvalidLexeme)
        }
    }
}

pub(super) fn address_digits(lexeme: &str) -> Result<(&[u8], u128), AddressError> {
    lexeme.strip_prefix("0x").map_or_else(
        || {
            let bytes = lexeme.as_bytes();
            let valid = bytes == b"0"
                || bytes
                    .first()
                    .is_some_and(|byte| matches!(byte, b'1'..=b'9'))
                    && valid_separated_digits(bytes, u8::is_ascii_digit);
            if valid {
                Ok((bytes, 10))
            } else {
                Err(AddressError::InvalidLexeme)
            }
        },
        |hex| {
            if valid_separated_digits(hex.as_bytes(), u8::is_ascii_hexdigit) {
                Ok((hex.as_bytes(), 16))
            } else {
                Err(AddressError::InvalidLexeme)
            }
        },
    )
}

fn valid_separated_digits(digits: &[u8], is_digit: fn(&u8) -> bool) -> bool {
    let Some(first) = digits.first() else {
        return false;
    };
    if !is_digit(first) {
        return false;
    }
    let mut previous_was_digit = true;
    for byte in digits.iter().skip(1) {
        if *byte == b'_' {
            if !previous_was_digit {
                return false;
            }
            previous_was_digit = false;
        } else if is_digit(byte) {
            previous_was_digit = true;
        } else {
            return false;
        }
    }
    previous_was_digit
}

pub(super) fn accumulate(
    digits: &[u8],
    radix: u128,
    underscores: bool,
) -> Result<u128, AccumulateError> {
    let mut value = 0_u128;
    for byte in digits {
        if *byte == b'_' && underscores {
            continue;
        }
        let digit = digit_value(*byte).ok_or(AccumulateError::Invalid)?;
        value = value
            .checked_mul(radix)
            .and_then(|partial| partial.checked_add(digit))
            .ok_or(AccumulateError::Overflow)?;
    }
    Ok(value)
}

fn digit_value(byte: u8) -> Option<u128> {
    match byte {
        b'0'..=b'9' => Some(u128::from(byte - b'0')),
        b'a'..=b'f' => Some(u128::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u128::from(byte - b'A') + 10),
        _ => None,
    }
}
