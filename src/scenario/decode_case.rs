use crate::{
    input::{SpannedMappingEntry, SpannedYamlKind, SpannedYamlNode},
    issue::IssuePath,
};

use super::{
    decode::ScenarioDecoder,
    decode_kind::{CaseKindTag, CasePayloadDraft},
    decode_support::{entries, find, invalid, invalid_observed, missing, unknown},
    decode_values::{has_duplicates, valid_name},
    model::{CaseName, ScenarioCase},
};

const COMMON_KEYS: &[&str] = &["name", "enabled", "kind", "window_sizes"];
const UNION_KEYS: &[&str] = &[
    "name",
    "enabled",
    "kind",
    "window_sizes",
    "base_bytes",
    "stride_bytes",
    "accesses",
    "schedule",
    "streams",
];

impl ScenarioDecoder {
    pub(super) fn cases(&mut self, node: &SpannedYamlNode) -> Option<Vec<ScenarioCase>> {
        let path = IssuePath::root().field("cases");
        let SpannedYamlKind::Sequence(items) = node.kind() else {
            self.issues.push(invalid(path, "sequence", node));
            return None;
        };
        if items.is_empty() {
            self.issues
                .push(invalid_observed(path, "non-empty sequence", "sequence"));
            return None;
        }
        let mut cases = Vec::with_capacity(items.len());
        let mut names = Vec::new();
        let mut complete = true;
        for (index, item) in items.iter().enumerate() {
            let case_path = path.clone().index(index);
            if let Some(name) = probe_name(item) {
                names.push(name);
            }
            match self.case(item, case_path) {
                Some(value) => cases.push(value),
                None => complete = false,
            }
        }
        if has_duplicates(names) {
            self.issues
                .push(invalid_observed(path, "unique values", "sequence"));
            complete = false;
        }
        complete.then_some(cases)
    }

    fn case(&mut self, node: &SpannedYamlNode, path: IssuePath) -> Option<ScenarioCase> {
        let Some(values) = entries(node) else {
            self.issues.push(invalid(path, "mapping", node));
            return None;
        };
        let kind_probe = CaseKindTag::probe(find(values, "kind"));
        let mut name = None;
        let mut enabled = Some(true);
        let mut windows = None;
        let mut payload = CasePayloadDraft::new(kind_probe);
        for entry in values {
            match entry.key().value() {
                "name" => {
                    name = self
                        .name_value(entry.value(), path.clone().field("name"))
                        .map(CaseName::new);
                }
                "enabled" => {
                    enabled = self.enabled(entry.value(), path.clone().field("enabled"));
                }
                "kind" => self.kind(entry.value(), path.clone().field("kind")),
                "window_sizes" => {
                    windows = self.window_sizes(entry.value(), path.clone().field("window_sizes"));
                }
                key if kind_probe.is_valid() => {
                    self.kind_field(&mut payload, entry, path.clone(), key);
                }
                key if UNION_KEYS.contains(&key) => {}
                key => self.issues.push(unknown(path.clone(), key)),
            }
        }
        for key in ["name", "kind"] {
            if find(values, key).is_none() {
                self.issues.push(missing(path.clone().field(key)));
            }
        }
        self.missing_kind_fields(values, kind_probe, &path);
        name.zip(enabled)
            .zip(payload.finish())
            .map(|((name, enabled), kind)| ScenarioCase {
                name,
                enabled,
                window_sizes: windows,
                kind,
            })
    }

    fn kind(&mut self, node: &SpannedYamlNode, path: IssuePath) {
        CaseKindTag::decode(self, node, path);
    }
}

fn probe_name(node: &SpannedYamlNode) -> Option<String> {
    let values = entries(node)?;
    let scalar = find(values, "name")?.as_scalar()?;
    (scalar.kind() == crate::input::ScalarKind::String
        && valid_name(scalar.value())
        && scalar.value().len() <= crate::input::limits::MAX_IDENTIFIER_UTF8_BYTES)
        .then(|| scalar.value().to_owned())
}

pub(super) fn allowed_common(key: &str) -> bool {
    COMMON_KEYS.contains(&key)
}

pub(super) fn present(values: &[SpannedMappingEntry], key: &str) -> bool {
    find(values, key).is_some()
}
