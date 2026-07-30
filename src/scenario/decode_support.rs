use crate::{
    input::{
        ScalarKind, ScalarStyle, SpannedMappingEntry, SpannedYamlKind, SpannedYamlNode,
        SpannedYamlScalar,
        scalar::{Address, GenericInteger, GenericIntegerError},
        yaml::canonical_value,
    },
    issue::{Issue, IssueCode, IssueOrderKey, IssuePath, IssuePhase, canonical_json_string},
};

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

pub(super) fn find<'entries>(
    values: &'entries [SpannedMappingEntry],
    key: &str,
) -> Option<&'entries SpannedYamlNode> {
    values
        .iter()
        .find(|entry| entry.key().value() == key)
        .map(SpannedMappingEntry::value)
}

pub(super) fn invalid(path: IssuePath, constraint: &str, node: &SpannedYamlNode) -> Issue {
    invalid_observed(path, constraint, &canonical_value(node))
}

pub(super) fn invalid_observed(path: IssuePath, constraint: &str, observed: &str) -> Issue {
    Issue::new(
        IssueCode::ScenarioInvalid,
        path,
        format!("expected {constraint}, observed {observed}"),
    )
    .with_order(IssueOrderKey::new(IssuePhase::Schema, 0, 0))
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

pub(super) fn parse_integer(node: &SpannedYamlNode) -> Result<(u128, String), &'static str> {
    let Some(value) = scalar(node).filter(|value| value.kind() == ScalarKind::Integer) else {
        return Err("integer");
    };
    let canonical = canonical_value(node);
    match GenericInteger::parse(value.value()) {
        Ok(integer) => Ok((integer.get(), canonical)),
        Err(GenericIntegerError::InvalidLexeme) => Err("plain integer"),
        Err(GenericIntegerError::Overflow | GenericIntegerError::OutOfRange) => {
            Ok((u128::MAX, canonical))
        }
    }
}

pub(super) fn parse_address(node: &SpannedYamlNode) -> Result<Address, ()> {
    let Some(value) = scalar(node).filter(|value| value.style() == ScalarStyle::Plain) else {
        return Err(());
    };
    Address::parse(value.value()).map_err(|_| ())
}

pub(super) fn parse_string(node: &SpannedYamlNode) -> Option<&SpannedYamlScalar> {
    scalar(node).filter(|value| value.kind() == ScalarKind::String)
}
