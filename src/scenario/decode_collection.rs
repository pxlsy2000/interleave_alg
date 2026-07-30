use crate::{
    input::{SpannedYamlNode, scalar::Address},
    issue::{IssuePath, canonical_json_string},
};

use super::{
    decode::ScenarioDecoder,
    decode_support::{invalid, invalid_observed, parse_string, sequence},
    decode_values::has_duplicates,
    model::Schedule,
};

impl ScenarioDecoder {
    pub(super) fn address_list(
        &mut self,
        node: &SpannedYamlNode,
        path: IssuePath,
    ) -> Option<Vec<Address>> {
        let Some(items) = sequence(node) else {
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
            match self.address(item, path.clone().index(index)) {
                Some(value) => values.push(value),
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

    pub(super) fn schedule(&mut self, node: &SpannedYamlNode, path: IssuePath) -> Option<Schedule> {
        let Some(value) = parse_string(node) else {
            self.issues.push(invalid(path, "string", node));
            return None;
        };
        if value.value() == "round_robin" {
            Some(Schedule::RoundRobin)
        } else {
            self.issues.push(invalid_observed(
                path,
                "one of [\"round_robin\"]",
                &canonical_json_string(value.value()),
            ));
            None
        }
    }
}
