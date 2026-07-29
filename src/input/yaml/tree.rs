//! Ordered, syntax-preserving YAML boundary values.

/// A source position with a byte-accurate offset.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourcePosition {
    byte_offset: usize,
    line: usize,
    column: usize,
}

impl SourcePosition {
    pub(super) const fn new(byte_offset: usize, line: usize, column: usize) -> Self {
        Self {
            byte_offset,
            line,
            column,
        }
    }

    /// Returns the zero-based byte offset in the original source.
    pub const fn byte_offset(self) -> usize {
        self.byte_offset
    }

    /// Returns the parser's one-based source line.
    pub const fn line(self) -> usize {
        self.line
    }

    /// Returns the parser's source column.
    pub const fn column(self) -> usize {
        self.column
    }
}

/// Inclusive-start, exclusive-end source range.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceSpan {
    start: SourcePosition,
    end: SourcePosition,
}

impl SourceSpan {
    pub(super) const fn new(start: SourcePosition, end: SourcePosition) -> Self {
        Self { start, end }
    }

    /// Returns the inclusive start position.
    pub const fn start(self) -> SourcePosition {
        self.start
    }

    /// Returns the exclusive end position.
    pub const fn end(self) -> SourcePosition {
        self.end
    }
}

/// Original YAML scalar presentation style.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScalarStyle {
    /// Unquoted plain scalar.
    Plain,
    /// Single-quoted scalar.
    SingleQuoted,
    /// Double-quoted scalar.
    DoubleQuoted,
    /// Literal block scalar.
    Literal,
    /// Folded block scalar.
    Folded,
}

/// YAML 1.2 scalar category needed by schema decoders.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScalarKind {
    /// String, including plain `yes` and `on`.
    String,
    /// Plain integer-shaped scalar.
    Integer,
    /// Plain YAML Core floating or non-finite spelling, without arithmetic.
    Float,
    /// Plain YAML Core boolean spelling.
    Boolean,
    /// Plain YAML null spelling.
    Null,
}

/// Decoded scalar content plus original style and source span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpannedYamlScalar {
    value: String,
    style: ScalarStyle,
    kind: ScalarKind,
    span: SourceSpan,
}

#[derive(Clone, Copy)]
pub(super) struct ScalarSyntax {
    pub(super) style: ScalarStyle,
    pub(super) kind: ScalarKind,
    pub(super) span: SourceSpan,
}

impl SpannedYamlScalar {
    pub(super) const fn new(value: String, syntax: ScalarSyntax) -> Self {
        Self {
            value,
            style: syntax.style,
            kind: syntax.kind,
            span: syntax.span,
        }
    }

    /// Returns decoded scalar content.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the original scalar presentation style.
    pub const fn style(&self) -> ScalarStyle {
        self.style
    }

    /// Returns the strict YAML 1.2 scalar category.
    pub const fn kind(&self) -> ScalarKind {
        self.kind
    }

    /// Returns the scalar source range.
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// One ordered string-keyed mapping entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpannedMappingEntry {
    key: SpannedYamlScalar,
    value: SpannedYamlNode,
}

impl SpannedMappingEntry {
    pub(super) const fn new(key: SpannedYamlScalar, value: SpannedYamlNode) -> Self {
        Self { key, value }
    }

    /// Returns the decoded string key.
    pub const fn key(&self) -> &SpannedYamlScalar {
        &self.key
    }

    /// Returns the entry value.
    pub const fn value(&self) -> &SpannedYamlNode {
        &self.value
    }
}

/// Syntax-preserving node payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpannedYamlKind {
    /// Scalar node.
    Scalar(SpannedYamlScalar),
    /// Ordered sequence items.
    Sequence(Vec<SpannedYamlNode>),
    /// Ordered string-keyed mapping entries.
    Mapping(Vec<SpannedMappingEntry>),
}

/// YAML node with a source range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpannedYamlNode {
    kind: SpannedYamlKind,
    span: SourceSpan,
}

impl SpannedYamlNode {
    pub(super) const fn new(kind: SpannedYamlKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }

    /// Returns the node payload.
    pub const fn kind(&self) -> &SpannedYamlKind {
        &self.kind
    }

    /// Returns the node source range.
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the scalar payload when this is a scalar node.
    pub const fn as_scalar(&self) -> Option<&SpannedYamlScalar> {
        match &self.kind {
            SpannedYamlKind::Scalar(scalar) => Some(scalar),
            SpannedYamlKind::Sequence(_) | SpannedYamlKind::Mapping(_) => None,
        }
    }
}

/// Exactly one parsed YAML document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpannedYamlDocument {
    root: SpannedYamlNode,
}

impl SpannedYamlDocument {
    pub(super) const fn new(root: SpannedYamlNode) -> Self {
        Self { root }
    }

    /// Returns the document root node.
    pub const fn root(&self) -> &SpannedYamlNode {
        &self.root
    }
}
