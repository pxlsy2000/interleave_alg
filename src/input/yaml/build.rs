use std::collections::BTreeSet;

use saphyr_parser::{Event, ScalarStyle as ParserScalarStyle, Span};

use crate::issue::IssuePath;

use super::{
    ScalarKind, ScalarStyle, SourceSpan, SpannedMappingEntry, SpannedYamlKind, SpannedYamlNode,
    SpannedYamlScalar,
    core::{scalar_kind, scalar_style},
    diagnostic::{PRIORITY_MERGE, PRIORITY_NON_STRING_KEY, Violation},
    location::PositionMap,
    path::child_path,
    tree::ScalarSyntax,
};

pub(super) struct BuildResult {
    pub(super) root: Option<SpannedYamlNode>,
    pub(super) violations: Vec<Violation>,
}

pub(super) fn build_root(
    events: &[(Event<'_>, Span)],
    positions: &PositionMap,
    root_index: usize,
) -> BuildResult {
    let mut builder = Builder {
        events,
        positions,
        violations: Vec::new(),
    };
    let mut cursor = root_index;
    let root = builder.node(&mut cursor, &IssuePath::root());
    BuildResult {
        root,
        violations: builder.violations,
    }
}

struct Builder<'events, 'source> {
    events: &'events [(Event<'source>, Span)],
    positions: &'events PositionMap,
    violations: Vec<Violation>,
}

impl Builder<'_, '_> {
    fn node(&mut self, cursor: &mut usize, path: &IssuePath) -> Option<SpannedYamlNode> {
        let (event, span) = self.events.get(*cursor)?;
        *cursor = cursor.saturating_add(1);
        match event {
            Event::Scalar(value, style, _, _) => {
                let scalar = self.scalar(value, *style, *span);
                Some(SpannedYamlNode::new(
                    SpannedYamlKind::Scalar(scalar),
                    self.positions.span(*span),
                ))
            }
            Event::SequenceStart(..) => Some(self.sequence(cursor, path, *span)),
            Event::MappingStart(..) => Some(self.mapping(cursor, path, *span)),
            Event::Alias(_) => Some(self.placeholder(*span)),
            Event::Nothing
            | Event::StreamStart
            | Event::StreamEnd
            | Event::DocumentStart(_)
            | Event::DocumentEnd
            | Event::SequenceEnd
            | Event::MappingEnd => None,
        }
    }

    fn sequence(&mut self, cursor: &mut usize, path: &IssuePath, start: Span) -> SpannedYamlNode {
        let mut items = Vec::new();
        let mut end = start;
        while let Some((event, span)) = self.events.get(*cursor) {
            if matches!(event, Event::SequenceEnd) {
                end = *span;
                *cursor = cursor.saturating_add(1);
                break;
            }
            if !is_node(event) {
                break;
            }
            let item_path = path.clone().index(items.len());
            if let Some(item) = self.node(cursor, &item_path) {
                end = *span;
                items.push(item);
            }
        }
        SpannedYamlNode::new(
            SpannedYamlKind::Sequence(items),
            self.combined_span(start, end),
        )
    }

    fn mapping(&mut self, cursor: &mut usize, path: &IssuePath, start: Span) -> SpannedYamlNode {
        let mut entries = Vec::new();
        let mut keys = BTreeSet::new();
        let mut end = start;
        while let Some((event, span)) = self.events.get(*cursor) {
            if matches!(event, Event::MappingEnd) {
                end = *span;
                *cursor = cursor.saturating_add(1);
                break;
            }
            if !is_node(event) {
                break;
            }
            let Some(key_node) = self.node(cursor, path) else {
                break;
            };
            let key = key_node.as_scalar().cloned();
            if let Some(valid_key) = key
                .as_ref()
                .filter(|scalar| scalar.kind() == ScalarKind::String)
            {
                if valid_key.style() == ScalarStyle::Plain && valid_key.value() == "<<" {
                    self.violations
                        .push(Violation::syntax(valid_key.span().start(), PRIORITY_MERGE));
                }
                if !keys.insert(valid_key.value().to_owned()) {
                    self.violations.push(Violation::duplicate(
                        valid_key.span().start(),
                        valid_key.value(),
                        path.clone().raw_key(valid_key.value()),
                    ));
                }
            } else {
                self.violations.push(Violation::syntax(
                    key_node.span().start(),
                    PRIORITY_NON_STRING_KEY,
                ));
            }
            let value_path = key
                .as_ref()
                .map_or_else(|| path.clone(), |scalar| child_path(path, scalar.value()));
            let Some(value) = self.node(cursor, &value_path) else {
                break;
            };
            end = *span;
            if let Some(valid_key) = key.filter(|scalar| scalar.kind() == ScalarKind::String) {
                entries.push(SpannedMappingEntry::new(valid_key, value));
            }
        }
        SpannedYamlNode::new(
            SpannedYamlKind::Mapping(entries),
            self.combined_span(start, end),
        )
    }

    fn scalar(&self, value: &str, style: ParserScalarStyle, span: Span) -> SpannedYamlScalar {
        let style = scalar_style(style);
        SpannedYamlScalar::new(
            value.to_owned(),
            ScalarSyntax {
                style,
                kind: scalar_kind(value, style),
                span: self.positions.span(span),
            },
        )
    }

    fn placeholder(&self, span: Span) -> SpannedYamlNode {
        let source_span = self.positions.span(span);
        let scalar = SpannedYamlScalar::new(
            String::new(),
            ScalarSyntax {
                style: ScalarStyle::Plain,
                kind: ScalarKind::String,
                span: source_span,
            },
        );
        SpannedYamlNode::new(SpannedYamlKind::Scalar(scalar), source_span)
    }

    fn combined_span(&self, start: Span, end: Span) -> SourceSpan {
        SourceSpan::new(
            self.positions.marker(start.start),
            self.positions.marker(end.end),
        )
    }
}

const fn is_node(event: &Event<'_>) -> bool {
    matches!(
        event,
        Event::Alias(_) | Event::Scalar(..) | Event::SequenceStart(..) | Event::MappingStart(..)
    )
}
