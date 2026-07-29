use saphyr_parser::{Marker, Span};

use super::{SourcePosition, SourceSpan};

pub(super) struct PositionMap {
    byte_offsets: Vec<usize>,
    source_bytes: usize,
    bom_bytes: usize,
}

impl PositionMap {
    pub(super) fn new(source: &str, bom_bytes: usize) -> Self {
        let mut byte_offsets: Vec<_> = source.char_indices().map(|(byte, _)| byte).collect();
        byte_offsets.push(source.len());
        Self {
            byte_offsets,
            source_bytes: source.len(),
            bom_bytes,
        }
    }

    pub(super) fn marker(&self, marker: Marker) -> SourcePosition {
        let relative = self
            .byte_offsets
            .get(marker.index())
            .copied()
            .unwrap_or(self.source_bytes);
        SourcePosition::new(
            relative.saturating_add(self.bom_bytes),
            marker.line(),
            marker.col(),
        )
    }

    pub(super) fn span(&self, span: Span) -> SourceSpan {
        SourceSpan::new(self.marker(span.start), self.marker(span.end))
    }

    pub(super) fn relative_byte_at_char(&self, char_offset: usize) -> usize {
        self.byte_offsets
            .get(char_offset)
            .copied()
            .unwrap_or(self.source_bytes)
    }

    pub(super) fn position_at_byte(&self, source: &str, relative: usize) -> SourcePosition {
        let prefix = source.get(..relative).unwrap_or(source);
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit_once('\n')
            .map_or_else(|| prefix.chars().count(), |(_, tail)| tail.chars().count());
        SourcePosition::new(relative.saturating_add(self.bom_bytes), line, column)
    }
}
