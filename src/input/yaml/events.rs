use saphyr_parser::{Event, Parser, Span};

use super::{
    diagnostic::{
        PRIORITY_ALIAS, PRIORITY_ANCHOR, PRIORITY_FLOW_ROOT, PRIORITY_SECOND_DOCUMENT,
        PRIORITY_SYNTAX, PRIORITY_TAG, Violation,
    },
    location::PositionMap,
};

pub(super) struct EventStream<'source> {
    pub(super) events: Vec<(Event<'source>, Span)>,
    pub(super) positions: PositionMap,
    pub(super) violations: Vec<Violation>,
    pub(super) document_count: usize,
    pub(super) root_indices: Vec<usize>,
}

pub(super) fn collect(text: &str, bom_bytes: usize) -> EventStream<'_> {
    let positions = PositionMap::new(text, bom_bytes);
    let mut events = Vec::new();
    let mut violations = Vec::new();
    let mut document_count = 0_usize;
    let mut second_document = None;
    let mut root_indices = Vec::new();
    let mut expects_root = false;
    let mut previous_end = 0_usize;

    for parsed in Parser::new_from_str(text) {
        let (event, span) = match parsed {
            Ok(located) => located,
            Err(error) => {
                violations.push(Violation::syntax(
                    positions.marker(*error.marker()),
                    PRIORITY_SYNTAX,
                ));
                break;
            }
        };
        if let Event::DocumentStart(_) = event {
            document_count = document_count.saturating_add(1);
            if document_count == 2 {
                second_document = Some(positions.marker(span.start));
            }
            expects_root = true;
        } else if expects_root && is_node_start(&event) {
            root_indices.push(events.len());
            if is_flow_root(text, &positions, (&event, span)) {
                violations.push(Violation::syntax(
                    positions.marker(span.start),
                    PRIORITY_FLOW_ROOT,
                ));
            }
            expects_root = false;
        }

        match &event {
            Event::Alias(_) => violations.push(Violation::syntax(
                positions.marker(span.start),
                PRIORITY_ALIAS,
            )),
            Event::Scalar(_, _, anchor, tag)
            | Event::SequenceStart(anchor, tag)
            | Event::MappingStart(anchor, tag) => {
                let region =
                    PropertyRegion::new(text, &positions, previous_end..span.start.index());
                if tag.is_some() {
                    violations.push(region.violation(b'!', PRIORITY_TAG));
                }
                if *anchor > 0 {
                    violations.push(region.violation(b'&', PRIORITY_ANCHOR));
                }
            }
            Event::Nothing
            | Event::StreamStart
            | Event::StreamEnd
            | Event::DocumentStart(_)
            | Event::DocumentEnd
            | Event::SequenceEnd
            | Event::MappingEnd => {}
        }
        previous_end = previous_end.max(span.end.index());
        events.push((event, span));
    }

    if let Some(position) = second_document {
        violations.push(Violation {
            position,
            priority: PRIORITY_SECOND_DOCUMENT,
            kind: super::diagnostic::ViolationKind::DocumentCount(document_count),
        });
    }
    EventStream {
        events,
        positions,
        violations,
        document_count,
        root_indices,
    }
}

const fn is_node_start(event: &Event<'_>) -> bool {
    matches!(
        event,
        Event::Alias(_) | Event::Scalar(..) | Event::SequenceStart(..) | Event::MappingStart(..)
    )
}

fn is_flow_root(text: &str, positions: &PositionMap, located: (&Event<'_>, Span)) -> bool {
    let (event, span) = located;
    if !matches!(event, Event::SequenceStart(..) | Event::MappingStart(..)) {
        return false;
    }
    let byte = positions.relative_byte_at_char(span.start.index());
    matches!(text.as_bytes().get(byte), Some(b'[' | b'{'))
}

struct PropertyRegion<'source> {
    text: &'source str,
    positions: &'source PositionMap,
    start: usize,
    end: usize,
}

impl<'source> PropertyRegion<'source> {
    fn new(
        text: &'source str,
        positions: &'source PositionMap,
        chars: std::ops::Range<usize>,
    ) -> Self {
        Self {
            text,
            positions,
            start: positions.relative_byte_at_char(chars.start),
            end: positions.relative_byte_at_char(chars.end),
        }
    }

    fn violation(&self, indicator: u8, priority: u8) -> Violation {
        let relative = self
            .text
            .as_bytes()
            .get(self.start..self.end)
            .and_then(|region| region.iter().position(|byte| *byte == indicator))
            .map_or(self.end, |offset| self.start.saturating_add(offset));
        Violation::syntax(
            self.positions.position_at_byte(self.text, relative),
            priority,
        )
    }
}
