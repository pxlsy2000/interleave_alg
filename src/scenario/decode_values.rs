use std::collections::BTreeSet;

use crate::{
    input::{ScalarKind, SpannedYamlNode, limits::MAX_IDENTIFIER_UTF8_BYTES, scalar::Address},
    issue::{IssuePath, canonical_json_string},
};

use super::{
    decode::ScenarioDecoder,
    decode_support::{
        entries, find, invalid, invalid_observed, missing, parse_address, parse_integer,
        parse_string, scalar, unknown,
    },
    model::{AccessCount, ScenarioDefaults, WindowSize},
};

pub(super) const MAX_ACCESS_COUNT: u128 = 10_000_000;

impl ScenarioDecoder {
    pub(super) fn schema_version(&mut self, node: &SpannedYamlNode) -> Option<()> {
        let path = IssuePath::root().field("schema_version");
        let (value, canonical) = match parse_integer(node) {
            Ok(parsed) => parsed,
            Err(constraint) => {
                self.issues.push(invalid(path, constraint, node));
                return None;
            }
        };
        if value == 1 {
            Some(())
        } else {
            self.issues
                .push(invalid_observed(path, "integer in [1,1]", &canonical));
            None
        }
    }

    pub(super) fn defaults(&mut self, node: &SpannedYamlNode) -> Option<ScenarioDefaults> {
        let path = IssuePath::root().field("defaults");
        let Some(values) = entries(node) else {
            self.issues.push(invalid(path, "mapping", node));
            return None;
        };
        let mut accesses = None;
        let mut windows = None;
        for entry in values {
            match entry.key().value() {
                "accesses" => {
                    accesses = self.access_count(entry.value(), path.clone().field("accesses"));
                }
                "window_sizes" => {
                    windows = self.window_sizes(entry.value(), path.clone().field("window_sizes"));
                }
                key => self.issues.push(unknown(path.clone(), key)),
            }
        }
        for key in ["accesses", "window_sizes"] {
            if find(values, key).is_none() {
                self.issues.push(missing(path.clone().field(key)));
            }
        }
        accesses
            .zip(windows)
            .map(|(accesses, window_sizes)| ScenarioDefaults {
                accesses,
                window_sizes,
            })
    }

    pub(super) fn access_count(
        &mut self,
        node: &SpannedYamlNode,
        path: IssuePath,
    ) -> Option<AccessCount> {
        let (value, canonical) = match parse_integer(node) {
            Ok(parsed) => parsed,
            Err(constraint) => {
                self.issues.push(invalid(path, constraint, node));
                return None;
            }
        };
        if !(1..=MAX_ACCESS_COUNT).contains(&value) {
            self.issues.push(invalid_observed(
                path,
                "integer in [1,10000000]",
                &canonical,
            ));
            return None;
        }
        u64::try_from(value).ok().map(AccessCount::new)
    }

    pub(super) fn window_sizes(
        &mut self,
        node: &SpannedYamlNode,
        path: IssuePath,
    ) -> Option<Vec<WindowSize>> {
        let Some(items) = super::decode_support::sequence(node) else {
            self.issues.push(invalid(path, "sequence", node));
            return None;
        };
        if items.is_empty() {
            self.issues
                .push(invalid_observed(path, "non-empty sequence", "sequence"));
            return None;
        }
        let mut values = Vec::with_capacity(items.len());
        let mut complete = true;
        for (index, item) in items.iter().enumerate() {
            match self.access_count(item, path.clone().index(index)) {
                Some(value) => values.push(WindowSize::new(value.get())),
                None => complete = false,
            }
        }
        if !complete {
            return None;
        }
        if has_duplicates(values.iter().map(|value| value.get())) {
            self.issues
                .push(invalid_observed(path, "unique values", "sequence"));
            return None;
        }
        Some(values)
    }

    pub(super) fn name_value(&mut self, node: &SpannedYamlNode, path: IssuePath) -> Option<String> {
        let Some(value) = parse_string(node) else {
            self.issues.push(invalid(path, "string", node));
            return None;
        };
        if !valid_name(value.value()) {
            self.issues.push(invalid_observed(
                path,
                "string matching \"[A-Za-z0-9][A-Za-z0-9._-]*\"",
                &canonical_json_string(value.value()),
            ));
            return None;
        }
        if value.value().len() > MAX_IDENTIFIER_UTF8_BYTES {
            self.issues.push(invalid_observed(
                path,
                "UTF-8 byte length <= 128",
                &value.value().len().to_string(),
            ));
            return None;
        }
        Some(value.value().to_owned())
    }

    pub(super) fn enabled(&mut self, node: &SpannedYamlNode, path: IssuePath) -> Option<bool> {
        let Some(value) = scalar(node).filter(|value| value.kind() == ScalarKind::Boolean) else {
            self.issues.push(invalid(path, "boolean", node));
            return None;
        };
        Some(value.value().eq_ignore_ascii_case("true"))
    }

    pub(super) fn address(&mut self, node: &SpannedYamlNode, path: IssuePath) -> Option<Address> {
        if let Ok(value) = parse_address(node) {
            Some(value)
        } else {
            self.issues.push(invalid(path, "plain address", node));
            None
        }
    }
}

pub(super) fn valid_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

pub(super) fn has_duplicates<T: Ord>(values: impl IntoIterator<Item = T>) -> bool {
    let mut seen = BTreeSet::new();
    values.into_iter().any(|value| !seen.insert(value))
}
