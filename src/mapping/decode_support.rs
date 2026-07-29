use crate::{
    input::{
        SpannedMappingEntry, SpannedYamlKind, SpannedYamlNode, SpannedYamlScalar,
        scalar::{GenericInteger, GenericIntegerError},
        yaml::canonical_value,
    },
    issue::{Issue, IssueCode, IssuePath, canonical_json_string},
};

pub(super) struct ParsedInteger {
    pub(super) value: Option<u128>,
    pub(super) canonical: String,
    pub(super) power_of_two: bool,
}

pub(super) fn entries(node: &SpannedYamlNode) -> Option<&[SpannedMappingEntry]> {
    match node.kind() {
        SpannedYamlKind::Mapping(entries) => Some(entries),
        SpannedYamlKind::Scalar(_) | SpannedYamlKind::Sequence(_) => None,
    }
}

pub(super) fn sequence(node: &SpannedYamlNode) -> Option<&[SpannedYamlNode]> {
    match node.kind() {
        SpannedYamlKind::Sequence(items) => Some(items),
        SpannedYamlKind::Scalar(_) | SpannedYamlKind::Mapping(_) => None,
    }
}

pub(super) const fn scalar(node: &SpannedYamlNode) -> Option<&SpannedYamlScalar> {
    node.as_scalar()
}

pub(super) fn parse_integer(node: &SpannedYamlNode) -> Result<ParsedInteger, &'static str> {
    let Some(scalar) = scalar(node) else {
        return Err("integer");
    };
    if scalar.kind() != crate::input::ScalarKind::Integer {
        return Err("integer");
    }
    let canonical = canonical_value(node);
    match GenericInteger::parse(scalar.value()) {
        Ok(value) => Ok(ParsedInteger {
            value: Some(value.get()),
            canonical,
            power_of_two: value.get().is_power_of_two(),
        }),
        Err(GenericIntegerError::Overflow) => Ok(ParsedInteger {
            value: None,
            canonical,
            power_of_two: lexeme_is_power_of_two(scalar.value()),
        }),
        Err(GenericIntegerError::InvalidLexeme | GenericIntegerError::OutOfRange) => {
            Err("plain integer")
        }
    }
}

pub(super) fn invalid(path: IssuePath, constraint: &str, node: &SpannedYamlNode) -> Issue {
    Issue::new(
        IssueCode::InputInvalidValue,
        path,
        format!("expected {constraint}, observed {}", canonical_value(node)),
    )
}

pub(super) fn invalid_observed(path: IssuePath, constraint: &str, observed: &str) -> Issue {
    Issue::new(
        IssueCode::InputInvalidValue,
        path,
        format!("expected {constraint}, observed {observed}"),
    )
}

pub(super) fn missing(path: IssuePath) -> Issue {
    invalid_observed(path, "required field", "missing")
}

pub(super) fn unknown(path: IssuePath, key: &str) -> Issue {
    Issue::new(
        IssueCode::InputUnknownField,
        path.raw_key(key),
        format!("unknown field {}", canonical_json_string(key)),
    )
}

pub(super) fn unsupported(path: &IssuePath, reason: &str) -> Issue {
    Issue::new(
        IssueCode::MappingUnsupported,
        path.clone(),
        format!("unsupported {}: {reason}", path.as_str()),
    )
}

pub(super) fn find<'entries>(
    entries: &'entries [SpannedMappingEntry],
    key: &str,
) -> Option<&'entries SpannedYamlNode> {
    entries
        .iter()
        .find(|entry| entry.key().value() == key)
        .map(SpannedMappingEntry::value)
}

fn lexeme_is_power_of_two(lexeme: &str) -> bool {
    if let Some(hex) = lexeme.strip_prefix("0x") {
        return hexadecimal_is_power_of_two(hex);
    }
    decimal_is_power_of_two(lexeme)
}

fn hexadecimal_is_power_of_two(digits: &str) -> bool {
    let mut found = false;
    for digit in digits.bytes() {
        let value = match digit {
            b'0'..=b'9' => digit - b'0',
            b'a'..=b'f' => digit - b'a' + 10,
            b'A'..=b'F' => digit - b'A' + 10,
            _ => return false,
        };
        if value != 0 {
            if found || !value.is_power_of_two() {
                return false;
            }
            found = true;
        }
    }
    found
}

fn decimal_is_power_of_two(digits: &str) -> bool {
    let mut value: Vec<u8> = digits
        .bytes()
        .map(|digit| digit.saturating_sub(b'0'))
        .collect();
    while value.first().is_some_and(|digit| *digit == 0) {
        value.remove(0);
    }
    if value.is_empty() {
        return false;
    }
    while value.len() > 1 || value.first().is_some_and(|digit| *digit > 1) {
        let mut carry = 0_u8;
        for digit in &mut value {
            let current = carry.saturating_mul(10).saturating_add(*digit);
            *digit = current / 2;
            carry = current % 2;
        }
        if carry != 0 {
            return false;
        }
        while value.first().is_some_and(|digit| *digit == 0) {
            value.remove(0);
        }
    }
    value == [1]
}
