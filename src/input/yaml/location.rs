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

    #[cfg(test)]
    fn position_at_byte_counted(&self, relative: usize) -> (SourcePosition, usize) {
        let (candidate, byte_comparisons) =
            partition_point_counted(&self.byte_offsets, |offset| offset < relative);
        let char_index = self.char_index(relative, candidate);
        let (line_end, line_comparisons) =
            partition_point_counted(&self.line_starts, |start| start <= char_index);
        (
            self.resolved_position(relative, char_index, line_end),
            byte_comparisons
                .saturating_add(line_comparisons)
                .saturating_add(1),
        )
    }
}

#[cfg(test)]
fn partition_point_counted(
    values: &[usize],
    mut is_left: impl FnMut(usize) -> bool,
) -> (usize, usize) {
    let mut left = 0;
    let mut right = values.len();
    let mut comparisons = 0_usize;
    while left < right {
        let middle = left + (right - left) / 2;
        comparisons = comparisons.saturating_add(1);
        if values.get(middle).copied().is_some_and(&mut is_left) {
            left = middle.saturating_add(1);
        } else {
            right = middle;
        }
    }
    (left, comparisons)
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::PositionMap;

    #[test]
    fn byte_position_lookup_work_is_bounded_per_query() {
        // Given
        const QUERY_COUNT: usize = 4_096;
        let source = "x".repeat(QUERY_COUNT);
        let positions = PositionMap::new(&source, 0);
        let mut lookup_work = 0_usize;

        // When
        for relative in 0..QUERY_COUNT {
            let actual = positions.position_at_byte(relative);
            let (counted, comparisons) = positions.position_at_byte_counted(relative);
            assert_eq!(actual, counted);
            lookup_work = lookup_work.saturating_add(comparisons);
        }

        // Then
        assert!(
            lookup_work <= QUERY_COUNT * 32,
            "position lookup used {lookup_work} index comparisons"
        );
    }

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

    #[test]
    fn repeated_tag_anchor_events_use_bounded_index_comparisons() -> Result<(), String> {
        // Given
        const ENTRY_COUNT: usize = 100_000;
        let mut source = String::new();
        for index in 0..ENTRY_COUNT {
            writeln!(source, "k{index}: !t &a{index} value").map_err(|error| error.to_string())?;
        }

        // When
        let stream = super::super::events::collect(&source, 0);
        let mut lookup_work = 0_usize;
        for violation in &stream.violations {
            let (counted, comparisons) = stream
                .positions
                .position_at_byte_counted(violation.position.byte_offset());
            assert_eq!(counted, violation.position);
            lookup_work = lookup_work.saturating_add(comparisons);
        }

        // Then
        assert_eq!(stream.violations.len(), ENTRY_COUNT * 2);
        assert!(
            lookup_work <= ENTRY_COUNT * 2 * 64,
            "{lookup_work} indexed comparisons exceeded the logarithmic bound"
        );
        Ok(())
    }
}
