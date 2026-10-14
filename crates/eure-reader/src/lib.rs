//! Reads Eure source text into an [`EureDocument`].
//!
//! `eure-parol` turns text into a concrete syntax tree; this crate interprets
//! that tree into the data model of `eure-document`.

mod interpreter;

use std::collections::HashMap;
use std::path::Path;

use eure_document::bytes::BytesDecodeError;
use eure_document::decimal::DecimalParseError;
use eure_document::document::constructor::ScopeError;
use eure_document::document::{EureDocument, InsertError, InsertErrorKind, NodeId};
use eure_document::identifier::IdentifierError;
use eure_document::path::PathSegment;
use eure_document::source::{BindSource, Comment, SectionBody, SourceDocument, SourceId, Trivia};
use eure_document::text::TextParseError;
use eure_document::value::ObjectKey;
use eure_parol::EureParseError;
use eure_tree::prelude::*;
use eure_tree::tree::InputSpan;
use thiserror::Error;

use crate::interpreter::CstInterpreter;

/// Origin tracking for document nodes and map keys.
///
/// This structure provides span resolution for error reporting:
/// - `definition`: Where the node's key/name is defined (for MissingRequiredField)
/// - `value`: The full value expression (for TypeMismatch, etc.)
/// - `key`: Maps (MapNodeId, ObjectKey) to the key's CstNodeId (for precise key spans)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginMap {
    /// Definition span (where the node's key/name is defined).
    /// Only the first definition is kept (via entry().or_insert()).
    pub definition: HashMap<NodeId, CstNodeId>,
    /// Value span (the full value expression).
    /// Later values overwrite earlier ones.
    pub value: HashMap<NodeId, CstNodeId>,
    /// (MapNodeId, ObjectKey) -> key's CstNodeId.
    /// Used for precise error spans on map keys.
    pub key: HashMap<(NodeId, ObjectKey), CstNodeId>,
    /// Direct span storage for special cases (e.g., split float keys).
    /// Used when we need precise sub-spans that don't correspond to a single CST node.
    pub key_span: HashMap<(NodeId, ObjectKey), InputSpan>,
    /// CST-based key span storage: (CstNodeId, ObjectKey) -> InputSpan.
    /// Used for error reporting when we only have the CST node ID, not the document NodeId.
    pub key_span_by_cst: HashMap<(CstNodeId, ObjectKey), InputSpan>,
}

impl OriginMap {
    /// Create a new empty OriginMap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a definition span for a node (typically the key).
    /// Only the first definition is kept.
    pub fn record_definition(&mut self, node_id: NodeId, cst_node_id: CstNodeId) {
        self.definition.entry(node_id).or_insert(cst_node_id);
    }

    /// Record a value span for a node (the full expression).
    /// Later values overwrite earlier ones.
    pub fn record_value(&mut self, node_id: NodeId, cst_node_id: CstNodeId) {
        self.value.insert(node_id, cst_node_id);
    }

    /// Record a map key origin.
    pub fn record_key(&mut self, map_node_id: NodeId, key: ObjectKey, cst_node_id: CstNodeId) {
        self.key.insert((map_node_id, key), cst_node_id);
    }

    /// Record a map key origin with direct InputSpan.
    /// Used for special cases where the key span doesn't correspond to a CST node.
    pub fn record_key_span(&mut self, map_node_id: NodeId, key: ObjectKey, span: InputSpan) {
        self.key_span.insert((map_node_id, key), span);
    }

    /// Record a map key origin with direct InputSpan, indexed by CST node ID.
    /// Used for error reporting when we only have the CST node ID.
    pub fn record_key_span_by_cst(
        &mut self,
        cst_node_id: CstNodeId,
        key: ObjectKey,
        span: InputSpan,
    ) {
        self.key_span_by_cst.insert((cst_node_id, key), span);
    }

    /// Get the value span for a node (the full value expression).
    /// Used for TypeMismatch and other value-related errors.
    pub fn get_value_span(&self, node_id: NodeId, cst: &Cst) -> Option<InputSpan> {
        self.value
            .get(&node_id)
            .and_then(|&cst_node_id| cst.span(cst_node_id))
    }

    /// Get the definition span for a node (where the key is defined).
    /// Used for MissingRequiredField errors to point to where the record is defined.
    pub fn get_definition_span(&self, node_id: NodeId, cst: &Cst) -> Option<InputSpan> {
        self.definition
            .get(&node_id)
            .and_then(|&cst_node_id| cst.span(cst_node_id))
    }

    /// Get the span for a specific map key.
    pub fn get_key_span(
        &self,
        map_node_id: NodeId,
        key: &ObjectKey,
        cst: &Cst,
    ) -> Option<InputSpan> {
        // First check direct span storage
        if let Some(&span) = self.key_span.get(&(map_node_id, key.clone())) {
            return Some(span);
        }
        // Fall back to CST node lookup
        self.key
            .get(&(map_node_id, key.clone()))
            .and_then(|&cst_node_id| cst.span(cst_node_id))
    }
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum InlineCodeError {
    #[error("Does not match InlineCode1 pattern")]
    InvalidInlineCode1Pattern,
    #[error("Does not match DelimCodeStart pattern")]
    InvalidDelimCodeStartPattern,
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CodeBlockError {
    #[error("Does not match CodeBlockStart pattern")]
    InvalidCodeBlockStartPattern,
    #[error(transparent)]
    ContentError(#[from] TextParseError),
}

#[derive(Debug, Error, Clone)]
pub enum DocumentConstructionError {
    #[error(transparent)]
    CstError(#[from] CstConstructError),
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(#[from] IdentifierError),
    #[error("Failed to parse integer: {0}")]
    InvalidInteger(String),
    #[error("Failed to parse float: {0}")]
    InvalidFloat(String),
    #[error("Document insert error: {error}")]
    DocumentInsert {
        error: InsertError,
        node_id: CstNodeId,
        /// The document NodeId where the error occurred (parent node for key errors)
        parent_node_id: Option<NodeId>,
    },
    #[error("Dynamic token not found: {0:?}")]
    DynamicTokenNotFound(DynamicTokenId),
    #[error("Failed to parse big integer: {0}")]
    InvalidBigInt(String),
    #[error("Invalid inline code at node {node_id:?}: {error}")]
    InvalidInlineCode {
        node_id: CstNodeId,
        error: InlineCodeError,
    },
    #[error("Invalid code block at node {node_id:?}: {error}")]
    InvalidCodeBlock {
        node_id: CstNodeId,
        error: CodeBlockError,
    },
    #[error("Invalid string key at node {node_id:?}: {error}")]
    InvalidStringKey {
        node_id: CstNodeId,
        error: TextParseError,
    },
    #[error("Invalid key type at node {node_id:?}")]
    InvalidKeyType { node_id: CstNodeId },
    #[error("Failed to end scope: {0}")]
    EndScope(#[from] ScopeError),
    #[error("Failed to parse tuple index: {value}")]
    InvalidTupleIndex { node_id: CstNodeId, value: String },
    #[error("Invalid literal string at node {node_id:?}")]
    InvalidLiteralStr { node_id: CstNodeId },
    #[error(
        "Float keys are not supported. Found '{value}'. Use integer keys like 'a.3.1' only when the pattern is <int>.<int>"
    )]
    InvalidFloatKey { node_id: CstNodeId, value: String },
    #[error("Invalid decimal literal: {error}")]
    InvalidDecimal {
        node_id: CstNodeId,
        error: DecimalParseError,
    },
    #[error("Invalid binary data: {error}")]
    InvalidBytes {
        node_id: CstNodeId,
        error: BytesDecodeError,
    },
    #[error("{element} cannot be a set element; use integers, strings, or tuples of them")]
    UnsupportedSetElement {
        node_id: CstNodeId,
        element: UnsupportedSetElement,
    },
}

/// Values that share the key grammar but are not allowed as set elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UnsupportedSetElement {
    #[error("A boolean")]
    Boolean,
    #[error("A hole")]
    Hole,
}

impl DocumentConstructionError {
    /// Get the span associated with this error, if available
    pub fn span(&self, cst: &Cst) -> Option<InputSpan> {
        match self {
            DocumentConstructionError::CstError(cst_error) => {
                let node_id = match cst_error {
                    CstConstructError::UnexpectedNode { node, .. } => Some(*node),
                    CstConstructError::UnexpectedExtraNode { node } => Some(*node),
                    CstConstructError::UnexpectedEndOfChildren { parent } => Some(*parent),
                    CstConstructError::UnexpectedEmptyChildren { node } => Some(*node),
                    CstConstructError::NodeIdNotFound { node } => Some(*node),
                    CstConstructError::Error(_) => None,
                };
                node_id.and_then(|id| cst.span(id))
            }
            DocumentConstructionError::DocumentInsert { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidInlineCode { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidCodeBlock { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidStringKey { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidKeyType { node_id } => cst.span(*node_id),
            DocumentConstructionError::InvalidFloatKey { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidDecimal { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidBytes { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::UnsupportedSetElement { node_id, .. } => cst.span(*node_id),
            _ => None,
        }
    }

    /// Get the span associated with this error, using OriginMap for precise key spans.
    /// This provides more accurate span information for key-related errors by consulting
    /// the OriginMap which tracks individual key spans, especially for split float keys.
    pub fn span_with_origin_map(&self, cst: &Cst, origins: &OriginMap) -> Option<InputSpan> {
        match self {
            DocumentConstructionError::DocumentInsert {
                error,
                node_id,
                parent_node_id,
            } => {
                // Try to get precise key span from OriginMap
                // First try: use parent_node_id if available
                if let (Some(parent_id), Some(key)) = (parent_node_id, self.extract_key(error))
                    && let Some(span) = origins.get_key_span(*parent_id, &key, cst)
                {
                    return Some(span);
                }

                // Second try: use CST node ID-based lookup
                if let Some(key) = self.extract_key(error)
                    && let Some(&span) = origins.key_span_by_cst.get(&(*node_id, key.clone()))
                {
                    return Some(span);
                }

                // Fallback to CST node span
                cst.span(*node_id)
            }
            // For other errors, use existing logic
            _ => self.span(cst),
        }
    }

    /// Extract the problematic key from an InsertError, if applicable
    fn extract_key(&self, error: &InsertError) -> Option<ObjectKey> {
        match &error.kind {
            InsertErrorKind::AlreadyAssigned { key } => Some(key.clone()),
            InsertErrorKind::BindingTargetHasValue
            | InsertErrorKind::ExpectedMap
            | InsertErrorKind::ExpectedArray => {
                // The last segment in the path is the problematic key
                error.path.0.last().and_then(|segment| {
                    if let PathSegment::Value(key) = segment {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
            }
            _ => None,
        }
    }
}

/// Error from reading Eure source text into a document.
#[derive(Debug, Error, Clone)]
pub enum ReadError {
    #[error(transparent)]
    Syntax(#[from] EureParseError),
    #[error(transparent)]
    Construction(#[from] DocumentConstructionError),
}

pub fn parse_to_document(input: &str, name: impl AsRef<Path>) -> Result<EureDocument, ReadError> {
    let tree = eure_parol::parse(input, name)?;
    Ok(cst_to_document(input, &tree)?)
}

pub fn parse_to_source_document(
    input: &str,
    name: impl AsRef<Path>,
) -> Result<SourceDocument, ReadError> {
    let tree = eure_parol::parse(input, name)?;
    Ok(cst_to_source_document(input, &tree)?)
}

pub fn cst_to_document(input: &str, cst: &Cst) -> Result<EureDocument, DocumentConstructionError> {
    let mut visitor = CstInterpreter::new(input);
    visitor.visit_root_handle(cst.root_handle(), cst)?;
    Ok(visitor.into_document())
}

pub fn cst_to_source_document(
    input: &str,
    cst: &Cst,
) -> Result<SourceDocument, DocumentConstructionError> {
    let mut visitor = CstInterpreter::new(input);
    cst.visit_from_root(&mut visitor)?;
    let mut source = visitor.into_source_document();
    annotate_source_trivia_from_input(&mut source, input);
    Ok(source)
}

fn annotate_source_trivia_from_input(source: &mut SourceDocument, input: &str) {
    let (chunks, trailing) = collect_statement_trivia_chunks(input);
    let mut chunks = chunks.into_iter();
    attach_source_trivia(source, source.root, &mut chunks);
    if source.source(source.root).trailing_trivia.is_empty() && !trailing.is_empty() {
        source.source_mut(source.root).trailing_trivia = trailing;
    }
}

fn collect_statement_trivia_chunks(input: &str) -> (Vec<Vec<Trivia>>, Vec<Trivia>) {
    let mut chunks = Vec::new();
    let mut pending = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            pending.push(Trivia::BlankLine);
            continue;
        }

        if let Some(comment) = parse_line_comment(trimmed) {
            pending.push(Trivia::Comment(Comment::Line(comment)));
            continue;
        }

        if let Some(comment) = parse_block_comment(trimmed) {
            pending.push(Trivia::Comment(Comment::Block(comment)));
            continue;
        }

        if trimmed == "}" || trimmed == "]" {
            continue;
        }

        chunks.push(std::mem::take(&mut pending));
    }

    (chunks, pending)
}

fn parse_line_comment(trimmed: &str) -> Option<String> {
    let comment = trimmed.strip_prefix("//")?;
    Some(comment.strip_prefix(' ').unwrap_or(comment).to_string())
}

fn parse_block_comment(trimmed: &str) -> Option<String> {
    let content = trimmed.strip_prefix("/*")?.strip_suffix("*/")?;
    Some(content.trim().to_string())
}

fn attach_source_trivia(
    source: &mut SourceDocument,
    source_id: SourceId,
    chunks: &mut impl Iterator<Item = Vec<Trivia>>,
) {
    let binding_len = source.source(source_id).bindings.len();
    for index in 0..binding_len {
        let nested = {
            let binding = &mut source.source_mut(source_id).bindings[index];
            if binding.trivia_before.is_empty() {
                binding.trivia_before = chunks.next().unwrap_or_default();
            }
            match binding.bind {
                BindSource::Block(nested) => Some(nested),
                _ => None,
            }
        };
        if let Some(nested) = nested {
            attach_source_trivia(source, nested, chunks);
        }
    }

    let section_len = source.source(source_id).sections.len();
    for index in 0..section_len {
        let block_nested = {
            let section = &mut source.source_mut(source_id).sections[index];
            if section.trivia_before.is_empty() {
                section.trivia_before = chunks.next().unwrap_or_default();
            }
            match &mut section.body {
                SectionBody::Items { bindings, .. } => {
                    for binding in bindings {
                        if binding.trivia_before.is_empty() {
                            binding.trivia_before = chunks.next().unwrap_or_default();
                        }
                    }
                    None
                }
                SectionBody::Block(nested) => Some(*nested),
            }
        };
        if let Some(nested) = block_nested {
            attach_source_trivia(source, nested, chunks);
        }
    }
}

/// Error type that includes partial OriginMap for precise error reporting.
#[derive(Debug, Clone)]
pub struct DocumentConstructionErrorWithOriginMap {
    pub error: DocumentConstructionError,
    pub partial_origins: OriginMap,
}

impl std::fmt::Display for DocumentConstructionErrorWithOriginMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for DocumentConstructionErrorWithOriginMap {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Parse CST to document and collect origin information for span resolution.
///
/// Returns `OriginMap` which includes both node origins and map key origins
/// for precise error reporting. On error, returns partial OriginMap collected
/// up to the point of failure for accurate error span resolution.
pub fn cst_to_document_and_origin_map(
    input: &str,
    cst: &Cst,
) -> Result<(EureDocument, OriginMap), Box<DocumentConstructionErrorWithOriginMap>> {
    let mut visitor = CstInterpreter::new(input);
    match visitor.visit_root_handle(cst.root_handle(), cst) {
        Ok(()) => Ok(visitor.into_document_and_origin_map()),
        Err(error) => {
            // Extract partial OriginMap even on error for precise error reporting
            let (_, partial_origins) = visitor.into_document_and_origin_map();
            Err(Box::new(DocumentConstructionErrorWithOriginMap {
                error,
                partial_origins,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_document::document::node::NodeValue;
    use eure_document::eure;
    use eure_document::plan::LayoutPlan;
    use eure_document::value::{ObjectKey, PrimitiveValue};
    use eure_fmt::format_source_document;
    use eure_tree::tree::CstFacade;

    fn parse_document(input: &str) -> EureDocument {
        let cst = eure_parol::parse(input, "<input>").unwrap();
        cst_to_document(input, &cst).unwrap()
    }

    /// Helper function to recursively find a node with a specific non-terminal kind
    fn find_node_by_kind(cst: &Cst, start: CstNodeId, kind: NonTerminalKind) -> Option<CstNodeId> {
        if let Some(CstNode::NonTerminal {
            kind: node_kind, ..
        }) = cst.node_data(start)
            && node_kind == kind
        {
            return Some(start);
        }

        for child in cst.children(start) {
            if let Some(found) = find_node_by_kind(cst, child, kind) {
                return Some(found);
            }
        }
        None
    }

    /// This test demonstrates that non-terminal spans include leading/trailing whitespace,
    /// and that get_shrunk_span correctly excludes them.
    #[test]
    fn test_shrunk_span_excludes_leading_trailing_trivia() {
        // Input with leading whitespace and newline before binding
        let input = "\n  foo = 1";
        let cst = eure_parol::parse(input, "<input>").unwrap();

        let root = cst.root();

        // Find the Binding non-terminal recursively
        let binding_node =
            find_node_by_kind(&cst, root, NonTerminalKind::Binding).expect("Should find Binding");

        // Get the original span (includes trivia)
        let original_span = cst.concrete_span(binding_node).unwrap();
        // Get the shrunk span (excludes trivia) - now using method syntax
        let shrunk_span = cst.span(binding_node).unwrap();

        // The original span should start at 0 (includes leading newline and whitespace)
        // Or at least include the whitespace before "foo"
        let original_text = original_span.as_str(input);
        let shrunk_text = shrunk_span.as_str(input);

        assert_eq!(original_text, "\n  foo = 1");

        // The shrunk text should be "foo = 1" (without leading whitespace)
        assert_eq!(
            shrunk_text, "foo = 1",
            "Shrunk span should be 'foo = 1', got '{}'",
            shrunk_text
        );
    }

    /// Test shrunk span with trailing comment
    #[test]
    fn test_shrunk_span_with_trailing_comment() {
        let input = "foo = 1 // comment\n";
        let cst = eure_parol::parse(input, "<input>").unwrap();

        let root = cst.root();
        let binding_node =
            find_node_by_kind(&cst, root, NonTerminalKind::Binding).expect("Should find Binding");

        // Get the original span of the root Eure node
        let original_span = cst.concrete_span(binding_node).unwrap();
        let shrunk_span = cst.span(binding_node).unwrap();

        let original_text = original_span.as_str(input);
        let shrunk_text = shrunk_span.as_str(input);

        // The shrunk span should not include trailing whitespace/comments
        // but should include all meaningful content
        assert_eq!(original_text, "foo = 1");
        assert_eq!(shrunk_text, "foo = 1");
    }

    /// Test that shrunk span works correctly for the Eure root node
    #[test]
    fn test_shrunk_span_for_eure_root() {
        // Input with leading and trailing whitespace
        let input = "  \n  foo = 1  \n  ";
        let cst = eure_parol::parse(input, "<input>").unwrap();

        let root = cst.root();

        // Find the Eure non-terminal
        let eure_node =
            find_node_by_kind(&cst, root, NonTerminalKind::Eure).expect("Should find Eure");

        let original_span = cst.concrete_span(eure_node).unwrap();
        let shrunk_span = cst.span(eure_node).unwrap();

        let original_text = original_span.as_str(input);
        let shrunk_text = shrunk_span.as_str(input);

        assert_eq!(original_text, "  \n  foo = 1");
        assert_eq!(
            shrunk_text.trim(),
            "foo = 1",
            "Shrunk span should be 'foo = 1'"
        );
    }

    /// Test that terminal spans are always returned (even for trivia)
    #[test]
    fn test_terminal_span_always_returned() {
        let input = "  foo = 1";
        let cst = eure_parol::parse(input, "<input>").unwrap();

        // Find a whitespace terminal
        fn find_whitespace(cst: &Cst, node_id: CstNodeId) -> Option<CstNodeId> {
            if let Some(CstNode::Terminal {
                kind: TerminalKind::Whitespace,
                ..
            }) = cst.node_data(node_id)
            {
                return Some(node_id);
            }
            for child in cst.children(node_id) {
                if let Some(found) = find_whitespace(cst, child) {
                    return Some(found);
                }
            }
            None
        }

        let ws_node = find_whitespace(&cst, cst.root()).expect("Should find whitespace");
        let ws_span = cst.span(ws_node).unwrap();

        // Whitespace terminal should still return its span
        assert_eq!(
            ws_span.as_str(input),
            "  ",
            "Terminal span should be returned even for trivia"
        );
    }

    #[test]
    fn test_hole_key_creates_partial_map() {
        let doc = parse_document("!x = 1");
        let root = doc.node(doc.get_root_id());
        assert!(matches!(root.content, NodeValue::PartialMap(_)));
    }

    #[test]
    fn test_anonymous_hole_key_creates_partial_map() {
        let doc = parse_document("! = 1");
        let root = doc.node(doc.get_root_id());
        assert!(matches!(root.content, NodeValue::PartialMap(_)));
    }

    #[test]
    fn test_hole_key_in_block_syntax() {
        let doc = parse_document("!a {\n  x = 1\n}");
        let root = doc.node(doc.get_root_id());
        assert!(matches!(root.content, NodeValue::PartialMap(_)));
    }

    #[test]
    fn test_hole_key_in_nested_path() {
        let doc = parse_document("a.!b = 1");
        let root = doc.node(doc.get_root_id());
        let a_node = match &root.content {
            NodeValue::Map(map) => map
                .get(&ObjectKey::String("a".into()))
                .copied()
                .map(|node_id| doc.node(node_id)),
            _ => None,
        };
        assert!(matches!(a_node, Some(node) if matches!(node.content, NodeValue::PartialMap(_))));
    }

    #[test]
    fn test_mixed_map_hole_and_resolved_keys() {
        let doc = parse_document("normal = 1\n!hole = 2");
        let root = doc.node(doc.get_root_id());
        assert!(matches!(root.content, NodeValue::PartialMap(_)));
    }

    #[test]
    fn test_hole_in_tuple_key_creates_partial_map() {
        let doc = parse_document("(1, !) = 1");
        let root = doc.node(doc.get_root_id());
        assert!(matches!(root.content, NodeValue::PartialMap(_)));
    }

    #[test]
    fn test_partial_map_structural_equality() {
        let input = "!x = 1";
        let doc1 = {
            let cst = eure_parol::parse(input, "<input>").unwrap();
            cst_to_document(input, &cst).unwrap()
        };
        let doc2 = {
            let cst = eure_parol::parse(input, "<input>").unwrap();
            cst_to_document(input, &cst).unwrap()
        };
        assert_eq!(doc1, doc2);
    }

    #[test]
    fn test_partial_map_different_labels_not_equal() {
        let input1 = "!x = 1";
        let input2 = "!y = 1";
        let doc1 = {
            let cst = eure_parol::parse(input1, "<input>").unwrap();
            cst_to_document(input1, &cst).unwrap()
        };
        let doc2 = {
            let cst = eure_parol::parse(input2, "<input>").unwrap();
            cst_to_document(input2, &cst).unwrap()
        };
        assert_ne!(doc1, doc2);
    }

    #[test]
    fn test_partial_map_format_round_trip() {
        let input = "!x = 1\nnormal = 2";
        let doc = parse_document(input);
        let source = LayoutPlan::auto(doc.clone()).expect("layout plan").emit();
        let formatted = format_source_document(&source);
        let reparsed = parse_document(&formatted);

        assert_eq!(reparsed, doc);
    }

    #[test]
    fn test_partial_tuple_key_format_round_trip() {
        let input = "(1, !) = 1";
        let doc = parse_document(input);
        let source = LayoutPlan::auto(doc.clone()).expect("layout plan").emit();
        let formatted = format_source_document(&source);
        let reparsed = parse_document(&formatted);

        assert_eq!(reparsed, doc);
    }

    #[test]
    fn test_parse_to_source_document_preserves_leading_comment() {
        let input = "// hello\nvalue = 1\n";
        let source = parse_to_source_document(input, "<input>").expect("source parse");

        assert_eq!(format_source_document(&source), input);
        assert_eq!(source.root_source().bindings.len(), 1);
        assert!(matches!(
            source.root_source().bindings[0].trivia_before.first(),
            Some(eure_document::source::Trivia::Comment(
                eure_document::source::Comment::Line(comment)
            )) if comment == "hello"
        ));
    }

    #[test]
    fn test_parse_to_source_document_preserves_inline_array_binding() {
        let input = "items = [1, 2, 3]\n";
        let source = parse_to_source_document(input, "<input>").expect("source parse");

        assert_eq!(format_source_document(&source), input);
    }

    // ==========================================================================
    // End-to-end parsing tests for the `[^]` (current-index) array marker.
    // ==========================================================================

    fn try_parse_document(input: &str) -> Result<EureDocument, DocumentConstructionError> {
        let cst = eure_parol::parse(input, "<input>").unwrap();
        cst_to_document(input, &cst)
    }

    #[test]
    fn test_current_index_merges_with_last_push() {
        // `users[].x = 1; users[^].y = 2` is equivalent to `users[0] = {x=1,y=2}`.
        let actual = parse_document("users[].x = 1\nusers[^].y = 2\n");
        let expected = parse_document("users[0].x = 1\nusers[0].y = 2\n");
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_current_index_in_nested_arrays() {
        let actual = parse_document(
            "orgs[].teams[].members[].name = \"Ada\"\n\
             orgs[^].teams[^].members[].name = \"Linus\"\n",
        );
        let expected = parse_document(
            "orgs[0].teams[0].members[0].name = \"Ada\"\n\
             orgs[0].teams[0].members[1].name = \"Linus\"\n",
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_current_index_without_prior_push_errors() {
        // `users[^]` with no matching `[]` push in scope must fail.
        let err = try_parse_document("users[^].name = \"Alice\"").unwrap_err();
        let msg = err.to_string();
        // The leading `users` path creates a hole, so ExpectedArray fires first.
        assert!(
            msg.contains("Expected array"),
            "expected ExpectedArray, got: {msg}"
        );
    }

    #[test]
    fn test_current_index_after_specific_index_errors() {
        // Seeding with a numeric index does not record a push, so `[^]` must
        // fail with ArrayCurrentOutOfScope.
        let err = try_parse_document("items[0].x = 1\nitems[^].y = 2\n").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no prior push"),
            "expected ArrayCurrentOutOfScope, got: {msg}"
        );
    }

    fn parse_document_error(input: &str) -> DocumentConstructionError {
        let cst = eure_parol::parse(input, "<input>").unwrap();
        cst_to_document(input, &cst).unwrap_err()
    }

    fn format_round_trip(doc: &EureDocument) -> EureDocument {
        let source = LayoutPlan::auto(doc.clone()).expect("layout plan").emit();
        parse_document(&format_source_document(&source))
    }

    // ---- Decimal ----

    #[test]
    fn test_unsuffixed_float_is_decimal() {
        let doc = parse_document("price = 1.10\nbig = 1e3\nneg = -0.5");
        assert_eq!(doc, eure!({ price = 1.10, big = 1e3, neg = -0.5 }));
        let price = doc.parse_record(doc.get_root_id()).unwrap();
        let price: &PrimitiveValue = price.parse_field("price").unwrap();
        assert_eq!(price, &PrimitiveValue::Decimal("1.10".parse().unwrap()));
    }

    #[test]
    fn test_suffixed_float_stays_float() {
        let doc = parse_document("a = 1.5f64\nb = 1.5f32\nc = inf");
        let inf = f64::INFINITY;
        assert_eq!(doc, eure!({ a = 1.5f64, b = 1.5f32, c = inf }));
    }

    #[test]
    fn test_decimal_keeps_precision() {
        let doc = parse_document("x = 0.1000000000000000000000001");
        let x: eure_document::decimal::Decimal = doc
            .parse_record(doc.get_root_id())
            .unwrap()
            .parse_field("x")
            .unwrap();
        assert_eq!(x.to_string(), "0.1000000000000000000000001");
    }

    #[test]
    fn test_decimal_parses_as_f64() {
        let doc = parse_document("x = 1.5");
        let x: f64 = doc
            .parse_record(doc.get_root_id())
            .unwrap()
            .parse_field("x")
            .unwrap();
        assert_eq!(x, 1.5);
    }

    #[test]
    fn test_decimal_format_round_trip() {
        let doc = parse_document("a = 1.10\nb = 1e3\nc = 0.005");
        assert_eq!(format_round_trip(&doc), doc);
    }

    // ---- Bytes ----

    #[test]
    fn test_base64_and_hex_are_bytes() {
        let hello = eure_document::bytes::Bytes(b"Hello".to_vec());
        let doc = parse_document("a = base64`SGVsbG8=`\nb = hex`48656c6c6f`");
        let hello2 = hello.clone();
        assert_eq!(doc, eure!({ a = hello, b = hello2 }));
    }

    #[test]
    fn test_bytes_equal_across_encodings() {
        assert_eq!(
            parse_document("a = base64`SGVsbG8=`"),
            parse_document("a = hex`48656C6C6F`")
        );
    }

    #[test]
    fn test_bytes_code_block_ignores_whitespace() {
        let hello = eure_document::bytes::Bytes(b"Hello".to_vec());
        let doc = parse_document("a = ```base64\nSGVs\nbG8=\n```\n");
        assert_eq!(doc, eure!({ a = hello }));
    }

    #[test]
    fn test_invalid_bytes_is_error() {
        assert!(matches!(
            parse_document_error("a = hex`48656`"),
            DocumentConstructionError::InvalidBytes {
                error: eure_document::bytes::BytesDecodeError::OddHexLength,
                ..
            }
        ));
        assert!(matches!(
            parse_document_error("a = base64`SGVsbG8`"),
            DocumentConstructionError::InvalidBytes {
                error: eure_document::bytes::BytesDecodeError::InvalidBase64Length,
                ..
            }
        ));
    }

    #[test]
    fn test_other_languages_stay_text() {
        let doc = parse_document("a = rust`fn main() {}`");
        assert_eq!(doc, eure!({ a = @code("rust", "fn main() {}") }));
    }

    #[test]
    fn test_bytes_format_round_trip() {
        let doc = parse_document("a = hex`00ff10`\nb = base64``");
        assert_eq!(format_round_trip(&doc), doc);
    }

    // ---- Set ----

    #[test]
    fn test_set_literal() {
        let doc = parse_document("tags = #{\"a\", 1, (2, \"b\")}\nempty = #{}");
        let tuple = ObjectKey::Tuple(eure_document::value::Tuple(vec![
            ObjectKey::from(2),
            ObjectKey::from("b"),
        ]));
        assert_eq!(doc, eure!({ tags = #{"a", 1, tuple}, empty = #{} }));
    }

    #[test]
    fn test_set_equality_ignores_order() {
        assert_eq!(
            parse_document("s = #{1, 2, 3}"),
            parse_document("s = #{3, 1, 2,}")
        );
        assert_ne!(
            parse_document("s = #{1, 2}"),
            parse_document("s = #{1, 2, 3}")
        );
    }

    #[test]
    fn test_set_differs_from_array() {
        assert_ne!(parse_document("s = #{1, 2}"), parse_document("s = [1, 2]"));
    }

    #[test]
    fn test_duplicate_set_element_is_error() {
        assert!(matches!(
            parse_document_error("s = #{1, \"a\", 1}"),
            DocumentConstructionError::DocumentInsert {
                error: InsertError {
                    kind: InsertErrorKind::DuplicateSetElement { .. },
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn test_boolean_and_hole_set_elements_are_errors() {
        assert!(matches!(
            parse_document_error("s = #{true}"),
            DocumentConstructionError::UnsupportedSetElement {
                element: UnsupportedSetElement::Boolean,
                ..
            }
        ));
        assert!(matches!(
            parse_document_error("s = #{!}"),
            DocumentConstructionError::UnsupportedSetElement {
                element: UnsupportedSetElement::Hole,
                ..
            }
        ));
    }

    #[test]
    fn test_set_parses_into_rust_sets() {
        let doc = parse_document("s = #{3, 1, 2}");
        let rec = doc.parse_record(doc.get_root_id()).unwrap();
        let set: std::collections::BTreeSet<i32> = rec.parse_field("s").unwrap();
        assert_eq!(set, [1, 2, 3].into_iter().collect());
    }

    #[test]
    fn test_set_format_round_trip() {
        let doc = parse_document("s = #{\"a b\", 1, (2, \"c\")}\nempty = #{}");
        assert_eq!(format_round_trip(&doc), doc);
    }
}
