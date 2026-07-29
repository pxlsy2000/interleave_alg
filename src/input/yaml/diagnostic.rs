use crate::issue::{Issue, IssueCode, IssuePath, canonical_json_string};

use super::SourcePosition;

pub(super) const PRIORITY_ENCODING: u8 = 1;
pub(super) const PRIORITY_SYNTAX: u8 = 2;
pub(super) const PRIORITY_FLOW_ROOT: u8 = 3;
pub(super) const PRIORITY_TAG: u8 = 4;
pub(super) const PRIORITY_ANCHOR: u8 = 5;
pub(super) const PRIORITY_ALIAS: u8 = 6;
pub(super) const PRIORITY_MERGE: u8 = 7;
pub(super) const PRIORITY_NON_STRING_KEY: u8 = 8;
pub(super) const PRIORITY_DUPLICATE: u8 = 9;
pub(super) const PRIORITY_SECOND_DOCUMENT: u8 = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ViolationKind {
    InvalidSyntax,
    Duplicate { key: String, path: IssuePath },
    DocumentCount(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Violation {
    pub(super) position: SourcePosition,
    pub(super) priority: u8,
    pub(super) kind: ViolationKind,
}

impl Violation {
    pub(super) const fn syntax(position: SourcePosition, priority: u8) -> Self {
        Self {
            position,
            priority,
            kind: ViolationKind::InvalidSyntax,
        }
    }

    pub(super) fn duplicate(position: SourcePosition, key: &str, path: IssuePath) -> Self {
        Self {
            position,
            priority: PRIORITY_DUPLICATE,
            kind: ViolationKind::Duplicate {
                key: key.to_owned(),
                path,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct YamlParseError {
    issue: Issue,
    position: SourcePosition,
}

impl YamlParseError {
    pub(super) fn from_violations(violations: Vec<Violation>) -> Option<Self> {
        violations
            .into_iter()
            .min_by_key(|violation| (violation.position.byte_offset(), violation.priority))
            .map(Self::from_violation)
    }

    pub(super) fn no_document(position: SourcePosition) -> Self {
        Self {
            issue: Issue::new(
                IssueCode::InputYamlParse,
                IssuePath::root(),
                "expected exactly one YAML document, found 0",
            ),
            position,
        }
    }

    pub(super) const fn issue(&self) -> &Issue {
        &self.issue
    }

    pub(super) const fn position(&self) -> SourcePosition {
        self.position
    }

    fn from_violation(violation: Violation) -> Self {
        let issue = match violation.kind {
            ViolationKind::InvalidSyntax => Issue::new(
                IssueCode::InputYamlParse,
                IssuePath::root(),
                "invalid YAML syntax",
            ),
            ViolationKind::Duplicate { key, path } => Issue::new(
                IssueCode::InputYamlParse,
                path,
                format!("duplicate key {}", canonical_json_string(&key)),
            ),
            ViolationKind::DocumentCount(count) => Issue::new(
                IssueCode::InputYamlParse,
                IssuePath::root(),
                format!("expected exactly one YAML document, found {count}"),
            ),
        };
        Self {
            issue,
            position: violation.position,
        }
    }
}
