mod interpreter;

use eros::Union as _;
pub use eure_document::data_model;
use eure_document::document::constructor::ScopeError;
pub use eure_document::document::node::NodeValue;
pub use eure_document::document::*;
pub use eure_document::identifier;
use eure_document::identifier::IdentifierError;
pub use eure_document::must_be;
pub use eure_document::parse;
pub use eure_document::path;
pub use eure_document::text;
use eure_document::text::TextParseError;
pub use eure_document::value;
pub use eure_document::write;
use eure_parol::EureParseError;

use crate::document::interpreter::CstInterpreter;
use eure_tree::prelude::*;
use eure_tree::tree::InputSpan;
use std::collections::HashMap;
use thiserror::Error;

use eure_document::path::PathSegment;
use eure_document::value::ObjectKey;

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

pub fn parse_to_document(
    input: &str,
) -> eros::UResult<EureDocument, (EureParseError, DocumentConstructionError)> {
    let tree = eure_parol::parse(input).union()?;
    let document = cst_to_document(input, &tree).union()?;
    Ok(document)
}

pub fn cst_to_document(input: &str, cst: &Cst) -> Result<EureDocument, DocumentConstructionError> {
    let mut visitor = CstInterpreter::new(input);
    visitor.visit_root_handle(cst.root_handle(), cst)?;
    Ok(visitor.into_document())
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
    use eure_tree::tree::CstFacade;

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
        let cst = eure_parol::parse(input).unwrap();

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
        let cst = eure_parol::parse(input).unwrap();

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
        let cst = eure_parol::parse(input).unwrap();

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
        let cst = eure_parol::parse(input).unwrap();

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
}
