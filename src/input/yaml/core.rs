use std::fmt::Write as _;

use saphyr_parser::ScalarStyle as ParserScalarStyle;

use super::{ScalarKind, ScalarStyle};
use crate::issue::canonical_json_string;

pub(super) const fn scalar_style(style: ParserScalarStyle) -> ScalarStyle {
    match style {
        ParserScalarStyle::Plain => ScalarStyle::Plain,
        ParserScalarStyle::SingleQuoted => ScalarStyle::SingleQuoted,
        ParserScalarStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
        ParserScalarStyle::Literal => ScalarStyle::Literal,
        ParserScalarStyle::Folded => ScalarStyle::Folded,
    }
}

pub(super) fn scalar_kind(value: &str, style: ScalarStyle) -> ScalarKind {
    if style != ScalarStyle::Plain {
        return ScalarKind::String;
    }
    match value {
        "~" | "null" | "Null" | "NULL" => ScalarKind::Null,
        "true" | "True" | "TRUE" | "false" | "False" | "FALSE" => ScalarKind::Boolean,
        integer if is_integer(integer) => ScalarKind::Integer,
        float if is_float(float) => ScalarKind::Float,
        _ => ScalarKind::String,
    }
}

fn is_integer(value: &str) -> bool {
    integer_lexeme(value).is_some()
}

pub(super) fn canonical_integer(value: &str) -> String {
    let Some(integer) = integer_lexeme(value) else {
        return canonical_json_string(value);
    };
    match integer {
        IntegerLexeme::Decimal { negative, digits } => canonical_decimal(negative, digits),
        IntegerLexeme::Octal(digits) => bounded_power_base_to_decimal(digits, 8),
        IntegerLexeme::Hexadecimal(digits) => bounded_power_base_to_decimal(digits, 16),
    }
}

enum IntegerLexeme<'source> {
    Decimal {
        negative: bool,
        digits: &'source str,
    },
    Octal(&'source str),
    Hexadecimal(&'source str),
}

fn integer_lexeme(value: &str) -> Option<IntegerLexeme<'_>> {
    let (negative, decimal) = match value.as_bytes().first() {
        Some(b'+') => (false, value.get(1..)?),
        Some(b'-') => (true, value.get(1..)?),
        _ => (false, value),
    };
    if !decimal.is_empty() && decimal.chars().all(|character| character.is_ascii_digit()) {
        return Some(IntegerLexeme::Decimal {
            negative,
            digits: decimal,
        });
    }
    if let Some(octal) = value.strip_prefix("0o").filter(|digits| {
        !digits.is_empty()
            && digits
                .chars()
                .all(|character| matches!(character, '0'..='7'))
    }) {
        return Some(IntegerLexeme::Octal(octal));
    }
    value
        .strip_prefix("0x")
        .filter(|digits| {
            !digits.is_empty()
                && digits
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
        })
        .map(IntegerLexeme::Hexadecimal)
}

fn canonical_decimal(negative: bool, digits: &str) -> String {
    let magnitude = digits.trim_start_matches('0');
    if magnitude.is_empty() {
        return "0".to_owned();
    }
    let mut canonical =
        String::with_capacity(magnitude.len().saturating_add(usize::from(negative)));
    if negative {
        canonical.push('-');
    }
    canonical.push_str(magnitude);
    canonical
}

fn bounded_power_base_to_decimal(digits: &str, radix: u32) -> String {
    const LIMB_BASE: u64 = 1_000_000_000;

    let multiplier = u64::from(radix);
    let mut limbs = vec![0_u64];
    for character in digits.chars() {
        let Some(digit) = character.to_digit(radix) else {
            return canonical_json_string(digits);
        };
        let mut carry = u64::from(digit);
        for limb in &mut limbs {
            let Some(next) = limb
                .checked_mul(multiplier)
                .and_then(|product| product.checked_add(carry))
            else {
                return canonical_json_string(digits);
            };
            *limb = next % LIMB_BASE;
            carry = next / LIMB_BASE;
        }
        if carry != 0 {
            limbs.push(carry);
        }
    }

    let mut reversed = limbs.iter().rev();
    let Some(first) = reversed.next() else {
        return "0".to_owned();
    };
    let mut canonical = first.to_string();
    for limb in reversed {
        if write!(&mut canonical, "{limb:09}").is_err() {
            return canonical_json_string(digits);
        }
    }
    canonical
}

fn is_float(value: &str) -> bool {
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        return true;
    }
    if matches!(value, ".nan" | ".NaN" | ".NAN") {
        return true;
    }

    let (mantissa, exponent) = split_exponent(unsigned);
    exponent.is_none_or(valid_exponent) && valid_mantissa(mantissa)
}

fn split_exponent(value: &str) -> (&str, Option<&str>) {
    let Some(index) = value.find(['e', 'E']) else {
        return (value, None);
    };
    if value[index + 1..].contains(['e', 'E']) {
        return ("", Some(""));
    }
    (&value[..index], Some(&value[index + 1..]))
}

fn valid_exponent(value: &str) -> bool {
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
}

fn valid_mantissa(value: &str) -> bool {
    let Some((whole, fraction)) = value.split_once('.') else {
        return !value.is_empty() && value.chars().all(|character| character.is_ascii_digit());
    };
    !value[whole.len() + 1..].contains('.')
        && (!whole.is_empty() || !fraction.is_empty())
        && whole.chars().all(|character| character.is_ascii_digit())
        && fraction.chars().all(|character| character.is_ascii_digit())
}
