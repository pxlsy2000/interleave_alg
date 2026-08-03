use saphyr_parser::{Marker, Span};

use super::{SourcePosition, SourceSpan};

pub(super) struct PositionMap {
    byte_offsets: Vec<usize>,
    line_starts: Vec<usize>,
    source_bytes: usize,
    bom_bytes: usize,
}

impl PositionMap {
    pub(super) fn new(source: &str, bom_bytes: usize) -> Self {
        let mut byte_offsets = Vec::new();
        let mut line_starts = vec![0];
        for (byte, character) in source.char_indices() {
            byte_offsets.push(byte);
            if character == '\n' {
                line_starts.push(byte_offsets.len());
            }
        }
        byte_offsets.push(source.len());
        Self {
            byte_offsets,
            line_starts,
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

    pub(super) fn position_at_byte(&self, relative: usize) -> SourcePosition {
        let candidate = self
            .byte_offsets
            .partition_point(|offset| *offset < relative);
        let char_index = self.char_index(relative, candidate);
        let line_end = self
            .line_starts
            .partition_point(|start| *start <= char_index);
        self.resolved_position(relative, char_index, line_end)
    }

    fn char_index(&self, relative: usize, candidate: usize) -> usize {
        let is_boundary = self.byte_offsets.get(candidate) == Some(&relative);
        if is_boundary {
            candidate
        } else {
            self.byte_offsets.len().saturating_sub(1)
        }
    }

    fn resolved_position(
        &self,
        relative: usize,
        char_index: usize,
        line_end: usize,
    ) -> SourcePosition {
        let line_index = line_end.saturating_sub(1);
        let line_start = self.line_starts.get(line_index).copied().unwrap_or(0);
        let line = line_index.saturating_add(1);
        let column = char_index.saturating_sub(line_start);
        SourcePosition::new(relative.saturating_add(self.bom_bytes), line, column)
    }
}

#[cfg(test)]
mod tests {
    use super::PositionMap;

    #[test]
    fn indexed_positions_match_simple_prefix_oracle_at_every_character_boundary()
    -> Result<(), String> {
        // Given
        let cases = [("", 0), ("ascii\nline\r\n", 0), ("é四𝄞\n后", 3)];

        // When / Then
        for (source, bom_bytes) in cases {
            let positions = PositionMap::new(source, bom_bytes);
            for relative in source
                .char_indices()
                .map(|(offset, _)| offset)
                .chain(std::iter::once(source.len()))
            {
                let prefix = source
                    .get(..relative)
                    .ok_or_else(|| format!("invalid test boundary {relative}"))?;
                let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
                let column = prefix
                    .rsplit_once('\n')
                    .map_or_else(|| prefix.chars().count(), |(_, tail)| tail.chars().count());
                let actual = positions.position_at_byte(relative);
                assert_eq!(actual.byte_offset(), relative + bom_bytes);
                assert_eq!(actual.line(), line);
                assert_eq!(actual.column(), column);
            }
        }
        Ok(())
    }
}
