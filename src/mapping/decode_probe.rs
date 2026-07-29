use crate::input::{
    SpannedMappingEntry, SpannedYamlNode,
    limits::{MAX_GRANULE_BYTES, MAX_TARGET_COUNT},
};

use super::{
    decode::{DimensionProbe, MappingDecoder},
    decode_support::{entries, find, parse_integer},
};

impl MappingDecoder {
    pub(super) fn probe_dimensions(root: &[SpannedMappingEntry]) -> DimensionProbe {
        let address = find(root, "address").and_then(entries);
        let width = address
            .and_then(|values| find(values, "width_bits"))
            .and_then(probe_u128)
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| (1..=64).contains(value));
        let granule = address
            .and_then(|values| find(values, "granule_bytes"))
            .and_then(probe_u128)
            .filter(|value| value.is_power_of_two())
            .filter(|value| width.is_some_and(|bits| *value <= (1_u128 << bits)))
            .filter(|value| *value <= MAX_GRANULE_BYTES);
        let offset = granule
            .and_then(crate::input::scalar::exact_log2)
            .and_then(|value| u8::try_from(value).ok());
        let line = width.zip(offset).and_then(|(bits, g)| bits.checked_sub(g));
        let target = find(root, "targets")
            .and_then(entries)
            .and_then(|values| find(values, "count"))
            .and_then(probe_u128)
            .filter(|value| value.is_power_of_two())
            .filter(|value| line.is_some_and(|n| *value <= (1_u128 << n)))
            .filter(|value| *value <= MAX_TARGET_COUNT);
        let target_bits = target
            .and_then(crate::input::scalar::exact_log2)
            .and_then(|value| u8::try_from(value).ok());
        let local_bits = line.zip(target_bits).and_then(|(n, r)| n.checked_sub(r));
        DimensionProbe {
            address_width: width,
            offset_bits: offset,
            line_bits: line,
            target_bits,
            local_bits,
        }
    }
}

fn probe_u128(node: &SpannedYamlNode) -> Option<u128> {
    parse_integer(node).ok()?.value
}
