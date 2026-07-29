use std::str;

use super::{
    SourcePosition,
    diagnostic::{PRIORITY_ENCODING, Violation},
};

const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";
const UTF16_LE_BOM: &[u8] = b"\xff\xfe";
const UTF16_BE_BOM: &[u8] = b"\xfe\xff";
const UTF32_LE_BOM: &[u8] = b"\xff\xfe\x00\x00";
const UTF32_BE_BOM: &[u8] = b"\x00\x00\xfe\xff";

pub(super) struct DecodedInput<'source> {
    pub(super) text: &'source str,
    pub(super) bom_bytes: usize,
    pub(super) violations: Vec<Violation>,
}

pub(super) fn decode(source: &[u8]) -> DecodedInput<'_> {
    if source.starts_with(UTF32_LE_BOM)
        || source.starts_with(UTF32_BE_BOM)
        || source.starts_with(UTF16_LE_BOM)
        || source.starts_with(UTF16_BE_BOM)
    {
        return DecodedInput {
            text: "",
            bom_bytes: 0,
            violations: vec![Violation::syntax(
                SourcePosition::new(0, 1, 0),
                PRIORITY_ENCODING,
            )],
        };
    }

    let bom_bytes = usize::from(source.starts_with(UTF8_BOM)) * UTF8_BOM.len();
    let payload = source.get(bom_bytes..).unwrap_or_default();
    let mut violations = Vec::new();
    let valid_bytes = match str::from_utf8(payload) {
        Ok(text) => text.len(),
        Err(error) => {
            let byte_offset = bom_bytes.saturating_add(error.valid_up_to());
            violations.push(Violation::syntax(
                position_for_byte(source, byte_offset),
                PRIORITY_ENCODING,
            ));
            error.valid_up_to()
        }
    };
    let valid_payload = payload.get(..valid_bytes).unwrap_or_default();
    for (relative, _) in valid_payload
        .windows(UTF8_BOM.len())
        .enumerate()
        .filter(|(_, bytes)| *bytes == UTF8_BOM)
    {
        let byte_offset = bom_bytes.saturating_add(relative);
        violations.push(Violation::syntax(
            position_for_byte(source, byte_offset),
            PRIORITY_ENCODING,
        ));
    }
    let text = str::from_utf8(valid_payload).unwrap_or_default();
    DecodedInput {
        text,
        bom_bytes,
        violations,
    }
}

fn position_for_byte(source: &[u8], byte_offset: usize) -> SourcePosition {
    let prefix = source.get(..byte_offset).unwrap_or(source);
    let valid = str::from_utf8(prefix).unwrap_or_default();
    let line = valid.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = valid
        .rsplit_once('\n')
        .map_or_else(|| valid.chars().count(), |(_, tail)| tail.chars().count());
    SourcePosition::new(byte_offset, line, column)
}
