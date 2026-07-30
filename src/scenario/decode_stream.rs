use crate::{
    input::{SpannedYamlKind, SpannedYamlNode},
    issue::IssuePath,
};

use super::{
    decode::ScenarioDecoder,
    decode_support::{entries, find, invalid, invalid_observed, missing, unknown},
    decode_values::{has_duplicates, valid_name},
    model::StreamName,
    model_kind::StreamScenario,
};

impl ScenarioDecoder {
    pub(super) fn streams(
        &mut self,
        node: &SpannedYamlNode,
        path: IssuePath,
    ) -> Option<Vec<StreamScenario>> {
        let SpannedYamlKind::Sequence(items) = node.kind() else {
            self.issues.push(invalid(path, "sequence", node));
            return None;
        };
        if items.is_empty() {
            self.issues
                .push(invalid_observed(path, "non-empty sequence", "sequence"));
            return None;
        }
        let mut streams = Vec::with_capacity(items.len());
        let mut names = Vec::new();
        let mut complete = true;
        for (index, item) in items.iter().enumerate() {
            let stream_path = path.clone().index(index);
            if let Some(name) = probe_stream_name(item) {
                names.push(name);
            }
            match self.stream(item, stream_path) {
                Some(value) => streams.push(value),
                None => complete = false,
            }
        }
        if has_duplicates(names) {
            self.issues
                .push(invalid_observed(path, "unique values", "sequence"));
            complete = false;
        }
        complete.then_some(streams)
    }

    fn stream(&mut self, node: &SpannedYamlNode, path: IssuePath) -> Option<StreamScenario> {
        let Some(values) = entries(node) else {
            self.issues.push(invalid(path, "mapping", node));
            return None;
        };
        let mut name = None;
        let mut base = None;
        let mut stride = None;
        let mut accesses = None;
        for entry in values {
            match entry.key().value() {
                "name" => {
                    name = self
                        .name_value(entry.value(), path.clone().field("name"))
                        .map(StreamName::new);
                }
                "base_bytes" => {
                    base = self.address(entry.value(), path.clone().field("base_bytes"));
                }
                "stride_bytes" => {
                    stride = self.address(entry.value(), path.clone().field("stride_bytes"));
                }
                "accesses" => {
                    accesses = self.access_count(entry.value(), path.clone().field("accesses"));
                }
                key => self.issues.push(unknown(path.clone(), key)),
            }
        }
        for key in ["name", "base_bytes", "stride_bytes", "accesses"] {
            if find(values, key).is_none() {
                self.issues.push(missing(path.clone().field(key)));
            }
        }
        name.zip(base).zip(stride).zip(accesses).map(
            |(((name, base_bytes), stride_bytes), accesses)| StreamScenario {
                name,
                base_bytes,
                stride_bytes,
                accesses,
            },
        )
    }
}

fn probe_stream_name(node: &SpannedYamlNode) -> Option<String> {
    let values = entries(node)?;
    let scalar = find(values, "name")?.as_scalar()?;
    (scalar.kind() == crate::input::ScalarKind::String
        && valid_name(scalar.value())
        && scalar.value().len() <= crate::input::limits::MAX_IDENTIFIER_UTF8_BYTES)
        .then(|| scalar.value().to_owned())
}
