use std::collections::BTreeSet;

use crate::{
    input::{ScalarKind, SpannedYamlKind, SpannedYamlNode, yaml::canonical_value},
    issue::IssuePath,
};

use super::{
    decode::{DimensionProbe, MappingDecoder, MatrixDraft},
    decode_support::{
        entries, find, invalid, invalid_observed, missing, parse_integer, scalar, sequence, unknown,
    },
    model::{LocalAddressRows, XorRow, XorTap},
};

#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    PreserveHigh,
    Explicit,
}

impl MappingDecoder {
    pub(super) fn matrix(
        &mut self,
        node: &SpannedYamlNode,
        probe: &DimensionProbe,
    ) -> Option<MatrixDraft> {
        let path = IssuePath::root().field("mapping");
        let Some(values) = entries(node) else {
            self.issues.push(invalid(path, "mapping", node));
            return None;
        };
        let mut target_rows = None;
        let mut local_rows = None;
        for entry in values {
            match entry.key().value() {
                "m" => target_rows = self.target_matrix(entry.value(), probe),
                "l" => local_rows = self.local_matrix(entry.value(), probe),
                key => self.issues.push(unknown(path.clone(), key)),
            }
        }
        for key in ["m", "l"] {
            if find(values, key).is_none() {
                self.issues.push(missing(path.clone().field(key)));
            }
        }
        target_rows
            .zip(local_rows)
            .map(|(target_rows, local_rows)| MatrixDraft {
                target_rows,
                local_rows,
            })
    }

    fn target_matrix(
        &mut self,
        node: &SpannedYamlNode,
        probe: &DimensionProbe,
    ) -> Option<Vec<XorRow>> {
        let path = IssuePath::root().field("mapping").field("m");
        let Some(values) = entries(node) else {
            self.issues.push(invalid(path, "mapping", node));
            return None;
        };
        let mut rows = None;
        for entry in values {
            match entry.key().value() {
                "rows" => {
                    rows = self.rows(
                        entry.value(),
                        path.clone().field("rows"),
                        probe.target_bits,
                        probe.line_bits,
                    );
                }
                key => self.issues.push(unknown(path.clone(), key)),
            }
        }
        if find(values, "rows").is_none() {
            self.issues.push(missing(path.field("rows")));
        }
        rows
    }

    fn local_matrix(
        &mut self,
        node: &SpannedYamlNode,
        probe: &DimensionProbe,
    ) -> Option<LocalAddressRows> {
        let path = IssuePath::root().field("mapping").field("l");
        let Some(values) = entries(node) else {
            self.issues.push(invalid(path, "mapping", node));
            return None;
        };
        let probed_mode = find(values, "mode").and_then(probe_mode);
        let mut mode = None;
        let mut rows = None;
        for entry in values {
            match entry.key().value() {
                "mode" => mode = self.mode(entry.value()),
                "rows" => match probed_mode {
                    Some(Mode::PreserveHigh) => self.issues.push(invalid_observed(
                        path.clone().field("rows"),
                        "field absent when mapping.l.mode=\"preserve_high\"",
                        &canonical_value(entry.value()),
                    )),
                    Some(Mode::Explicit) => {
                        rows = self.rows(
                            entry.value(),
                            path.clone().field("rows"),
                            probe.local_bits,
                            probe.line_bits,
                        );
                    }
                    None => {}
                },
                key => self.issues.push(unknown(path.clone(), key)),
            }
        }
        if find(values, "mode").is_none() {
            self.issues.push(missing(path.clone().field("mode")));
        }
        match mode {
            Some(Mode::PreserveHigh) => Some(LocalAddressRows::PreserveHigh),
            Some(Mode::Explicit) => {
                if find(values, "rows").is_none() {
                    self.issues.push(invalid_observed(
                        path.field("rows"),
                        "field present when mapping.l.mode=\"explicit\"",
                        "missing",
                    ));
                }
                rows.map(LocalAddressRows::Explicit)
            }
            None => None,
        }
    }

    fn mode(&mut self, node: &SpannedYamlNode) -> Option<Mode> {
        let path = IssuePath::root().field("mapping").field("l").field("mode");
        let Some(value) = scalar(node).filter(|value| value.kind() == ScalarKind::String) else {
            self.issues.push(invalid(path, "string", node));
            return None;
        };
        match value.value() {
            "preserve_high" => Some(Mode::PreserveHigh),
            "explicit" => Some(Mode::Explicit),
            _ => {
                self.issues.push(invalid_observed(
                    path,
                    "one of [\"preserve_high\",\"explicit\"]",
                    &canonical_value(node),
                ));
                None
            }
        }
    }

    fn rows(
        &mut self,
        node: &SpannedYamlNode,
        path: IssuePath,
        expected_rows: Option<u8>,
        line_bits: Option<u8>,
    ) -> Option<Vec<XorRow>> {
        let Some(rows) = sequence(node) else {
            self.issues.push(invalid(path, "sequence", node));
            return None;
        };
        if expected_rows.is_some_and(|expected| usize::from(expected) != rows.len()) {
            self.issues.push(invalid_observed(
                path.clone(),
                &format!("sequence length {}", expected_rows.unwrap_or_default()),
                "sequence",
            ));
        }
        let decoded: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(index, row)| self.row(row, path.clone().index(index), line_bits))
            .collect();
        decoded.into_iter().collect()
    }

    fn row(
        &mut self,
        node: &SpannedYamlNode,
        path: IssuePath,
        line_bits: Option<u8>,
    ) -> Option<XorRow> {
        let Some(taps) = sequence(node) else {
            self.issues.push(invalid(path, "sequence", node));
            return None;
        };
        let mut seen = BTreeSet::new();
        if taps
            .iter()
            .filter_map(|tap| parse_integer(tap).ok())
            .any(|tap| !seen.insert(tap.canonical))
        {
            self.issues
                .push(invalid_observed(path.clone(), "unique values", "sequence"));
        }
        let decoded: Vec<_> = taps
            .iter()
            .enumerate()
            .map(|(index, tap)| self.tap(tap, path.clone().index(index), line_bits))
            .collect();
        decoded
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .map(|taps| XorRow::new(taps.into_iter().map(XorTap::new).collect()))
    }

    fn tap(
        &mut self,
        node: &SpannedYamlNode,
        path: IssuePath,
        line_bits: Option<u8>,
    ) -> Option<u8> {
        let parsed = match parse_integer(node) {
            Ok(parsed) => parsed,
            Err(constraint) => {
                self.issues.push(invalid(path, constraint, node));
                return None;
            }
        };
        let Some(maximum) = line_bits.and_then(|bits| bits.checked_sub(1)) else {
            return parsed.value.and_then(|value| u8::try_from(value).ok());
        };
        match parsed.value.and_then(|value| u8::try_from(value).ok()) {
            Some(value) if value <= maximum => Some(value),
            Some(_) | None => {
                self.issues.push(invalid_observed(
                    path,
                    &format!("integer in [0,{maximum}]"),
                    &parsed.canonical,
                ));
                None
            }
        }
    }
}

fn probe_mode(node: &SpannedYamlNode) -> Option<Mode> {
    let SpannedYamlKind::Scalar(value) = node.kind() else {
        return None;
    };
    if value.kind() != ScalarKind::String {
        return None;
    }
    match value.value() {
        "preserve_high" => Some(Mode::PreserveHigh),
        "explicit" => Some(Mode::Explicit),
        _ => None,
    }
}
