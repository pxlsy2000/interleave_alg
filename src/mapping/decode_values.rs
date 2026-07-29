use crate::{
    input::{
        ScalarKind, SpannedYamlNode,
        limits::{MAX_GRANULE_BYTES, MAX_IDENTIFIER_UTF8_BYTES, MAX_TARGET_COUNT},
        scalar::AddressWidth,
    },
    issue::{IssuePath, canonical_json_string},
};

use super::{
    decode::{AddressDraft, DimensionProbe, MappingDecoder, TargetDraft},
    decode_support::{
        ParsedInteger, entries, find, invalid, invalid_observed, missing, parse_integer, scalar,
        unknown, unsupported,
    },
    model::{GranuleBytes, MappingName, TargetCount},
};

impl MappingDecoder {
    pub(super) fn schema_version(&mut self, node: &SpannedYamlNode) -> Option<()> {
        let path = IssuePath::root().field("schema_version");
        let parsed = match parse_integer(node) {
            Ok(parsed) => parsed,
            Err(constraint) => {
                self.issues.push(invalid(path, constraint, node));
                return None;
            }
        };
        if parsed.value == Some(1) {
            Some(())
        } else {
            self.issues.push(invalid_observed(
                path,
                "integer in [1,1]",
                &parsed.canonical,
            ));
            None
        }
    }

    pub(super) fn name(&mut self, node: &SpannedYamlNode) -> Option<MappingName> {
        let path = IssuePath::root().field("name");
        let Some(value) = scalar(node).filter(|value| value.kind() == ScalarKind::String) else {
            self.issues.push(invalid(path, "string", node));
            return None;
        };
        if value.value().is_empty()
            || value.value().chars().any(|character| {
                character.is_control() || matches!(character, '\u{2028}' | '\u{2029}')
            })
        {
            self.issues.push(invalid_observed(
                path,
                "non-empty string without control or line-separator characters",
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
        Some(MappingName::new(value.value().to_owned()))
    }

    pub(super) fn address(
        &mut self,
        node: &SpannedYamlNode,
        probe: &DimensionProbe,
    ) -> Option<AddressDraft> {
        let path = IssuePath::root().field("address");
        let Some(values) = entries(node) else {
            self.issues.push(invalid(path, "mapping", node));
            return None;
        };
        let mut width = None;
        let mut granule = None;
        for entry in values {
            match entry.key().value() {
                "width_bits" => width = self.width(entry.value()),
                "granule_bytes" => granule = self.granule(entry.value(), probe),
                key => self.issues.push(unknown(path.clone(), key)),
            }
        }
        for key in ["width_bits", "granule_bytes"] {
            if find(values, key).is_none() {
                self.issues.push(missing(path.clone().field(key)));
            }
        }
        width
            .zip(granule)
            .map(|(width, granule)| AddressDraft { width, granule })
    }

    pub(super) fn targets(
        &mut self,
        node: &SpannedYamlNode,
        probe: &DimensionProbe,
    ) -> Option<TargetDraft> {
        let path = IssuePath::root().field("targets");
        let Some(values) = entries(node) else {
            self.issues.push(invalid(path, "mapping", node));
            return None;
        };
        let mut count = None;
        for entry in values {
            match entry.key().value() {
                "count" => count = self.target_count(entry.value(), probe),
                key => self.issues.push(unknown(path.clone(), key)),
            }
        }
        if find(values, "count").is_none() {
            self.issues.push(missing(path.field("count")));
        }
        count.map(|count| TargetDraft { count })
    }

    fn width(&mut self, node: &SpannedYamlNode) -> Option<AddressWidth> {
        let path = IssuePath::root().field("address").field("width_bits");
        let parsed = self.bounded_integer(node, path, "integer in [1,64]", 1, 64)?;
        let value = u8::try_from(parsed).ok()?;
        AddressWidth::new(value).ok()
    }

    fn granule(&mut self, node: &SpannedYamlNode, probe: &DimensionProbe) -> Option<GranuleBytes> {
        let path = IssuePath::root().field("address").field("granule_bytes");
        let parsed = self.cross_family_integer(node, path.clone())?;
        if !parsed.power_of_two {
            self.issues
                .push(unsupported(&path, "granule size is not a power of two"));
            return None;
        }
        if let Some(width) = probe.address_width {
            let maximum = 1_u128 << width;
            if parsed.value.is_none_or(|value| value > maximum) {
                self.issues.push(invalid_observed(
                    path,
                    &format!("integer <= {maximum}"),
                    &parsed.canonical,
                ));
                return None;
            }
        }
        if parsed.value.is_none_or(|value| value > MAX_GRANULE_BYTES) {
            self.issues.push(unsupported(
                &path,
                "granule size exceeds v1 limit 4503599627370496",
            ));
            return None;
        }
        parsed
            .value
            .and_then(|value| u64::try_from(value).ok())
            .map(GranuleBytes::new)
    }

    fn target_count(
        &mut self,
        node: &SpannedYamlNode,
        probe: &DimensionProbe,
    ) -> Option<TargetCount> {
        let path = IssuePath::root().field("targets").field("count");
        let parsed = self.cross_family_integer(node, path.clone())?;
        if !parsed.power_of_two {
            self.issues
                .push(unsupported(&path, "target count is not a power of two"));
            return None;
        }
        if let Some(line_bits) = probe.line_bits {
            let maximum = 1_u128 << line_bits;
            if parsed.value.is_none_or(|value| value > maximum) {
                self.issues.push(invalid_observed(
                    path,
                    &format!("integer <= {maximum}"),
                    &parsed.canonical,
                ));
                return None;
            }
        }
        if parsed.value.is_none_or(|value| value > MAX_TARGET_COUNT) {
            self.issues
                .push(unsupported(&path, "target count exceeds v1 limit 65536"));
            return None;
        }
        parsed
            .value
            .and_then(|value| u32::try_from(value).ok())
            .map(TargetCount::new)
    }

    fn cross_family_integer(
        &mut self,
        node: &SpannedYamlNode,
        path: IssuePath,
    ) -> Option<ParsedInteger> {
        match parse_integer(node) {
            Ok(parsed) => Some(parsed),
            Err(constraint) => {
                self.issues.push(invalid(path, constraint, node));
                None
            }
        }
    }

    fn bounded_integer(
        &mut self,
        node: &SpannedYamlNode,
        path: IssuePath,
        constraint: &str,
        minimum: u128,
        maximum: u128,
    ) -> Option<u128> {
        let parsed = match parse_integer(node) {
            Ok(parsed) => parsed,
            Err(gate) => {
                self.issues.push(invalid(path, gate, node));
                return None;
            }
        };
        match parsed.value {
            Some(value) if (minimum..=maximum).contains(&value) => Some(value),
            Some(_) | None => {
                self.issues
                    .push(invalid_observed(path, constraint, &parsed.canonical));
                None
            }
        }
    }
}
