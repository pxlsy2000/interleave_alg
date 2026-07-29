use saphyr_parser::ScalarStyle as ParserScalarStyle;

use super::{ScalarKind, ScalarStyle};

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
        "true" | "True" | "TRUE" | "false" | "False" | "FALSE" => ScalarKind::Boolean,
        "~" | "null" | "Null" | "NULL" => ScalarKind::Null,
        integer if is_integer(integer) => ScalarKind::Integer,
        float if is_float(float) => ScalarKind::Float,
        _ => ScalarKind::String,
    }
}

fn is_integer(value: &str) -> bool {
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    if let Some(hex) = unsigned.strip_prefix("0x") {
        return !hex.is_empty() && hex.chars().all(|character| character.is_ascii_hexdigit());
    }
    if let Some(octal) = unsigned.strip_prefix("0o") {
        return !octal.is_empty()
            && octal
                .chars()
                .all(|character| matches!(character, '0'..='7'));
    }
    unsigned == "0"
        || unsigned
            .strip_prefix(|character: char| matches!(character, '1'..='9'))
            .is_some_and(|digits| digits.chars().all(|character| character.is_ascii_digit()))
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
