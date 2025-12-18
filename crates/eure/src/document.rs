mod interpreter;

use eros::Union as _;
use eure_document::document::constructor::ScopeError;
pub use eure_document::document::node::NodeValue;
pub use eure_document::document::*;
use eure_document::identifier::IdentifierError;
pub use eure_document::parse;
use eure_document::text::TextParseError;
use eure_parol::EureParseError;

use crate::document::interpreter::CstInterpreter;
use eure_tree::node_kind::{NonTerminalKind, TerminalKind};
use eure_tree::prelude::*;
use eure_tree::tree::{InputSpan, ViewConstructionError};
use std::collections::HashMap;
use thiserror::Error;

/// Type alias for ViewConstructionError with Eure-specific types
pub type EureViewConstructionError = ViewConstructionError<TerminalKind, NonTerminalKind>;

use eure_document::value::ObjectKey;

/// Origin tracking for document nodes and map keys.
///
/// This structure provides span resolution for error reporting:
/// - `node`: Maps document NodeId to CST origins (for general node spans)
/// - `key`: Maps (MapNodeId, ObjectKey) to the key's CstNodeId (for precise key spans)
#[derive(Debug, Clone, Default)]
pub struct OriginMap {
    /// NodeId -> CstNodeId origins.
    /// A node can have multiple origins (e.g., created via key + value).
    pub node: HashMap<NodeId, Vec<CstNodeId>>,
    /// (MapNodeId, ObjectKey) -> key's CstNodeId.
    /// Used for precise error spans on map keys.
    pub key: HashMap<(NodeId, ObjectKey), CstNodeId>,
}

impl OriginMap {
    /// Create a new empty OriginMap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a node origin.
    pub fn record_node(&mut self, node_id: NodeId, cst_node_id: CstNodeId) {
        self.node.entry(node_id).or_default().push(cst_node_id);
    }

    /// Record a map key origin.
    pub fn record_key(&mut self, map_node_id: NodeId, key: ObjectKey, cst_node_id: CstNodeId) {
        self.key.insert((map_node_id, key), cst_node_id);
    }

    /// Get the first origin span for a node.
    pub fn get_node_span(&self, node_id: NodeId, cst: &Cst) -> Option<InputSpan> {
        self.node
            .get(&node_id)
            .and_then(|origins| origins.first())
            .and_then(|&cst_node_id| cst.span(cst_node_id))
    }

    /// Get the span for a specific map key.
    pub fn get_key_span(
        &self,
        map_node_id: NodeId,
        key: &ObjectKey,
        cst: &Cst,
    ) -> Option<InputSpan> {
        self.key
            .get(&(map_node_id, key.clone()))
            .and_then(|&cst_node_id| cst.span(cst_node_id))
    }
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum InlineCodeError {
    #[error("Does not match InlineCode1 pattern")]
    InvalidInlineCode1Pattern,
    #[error("Does not match InlineCodeStart2 pattern")]
    InvalidInlineCodeStart2Pattern,
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
    CstError(#[from] EureViewConstructionError),
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
}

impl DocumentConstructionError {
    /// Get the span associated with this error, if available
    pub fn span(&self, cst: &Cst) -> Option<InputSpan> {
        match self {
            DocumentConstructionError::CstError(view_error) => {
                let node_id = view_error.node_id();
                cst.span(node_id)
            }
            DocumentConstructionError::DocumentInsert { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidInlineCode { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidCodeBlock { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidStringKey { node_id, .. } => cst.span(*node_id),
            DocumentConstructionError::InvalidKeyType { node_id } => cst.span(*node_id),
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

/// Parse CST to document and collect origin information for span resolution.
///
/// Returns `OriginMap` which includes both node origins and map key origins
/// for precise error reporting.
pub fn cst_to_document_and_origin_map(
    input: &str,
    cst: &Cst,
) -> Result<(EureDocument, OriginMap), DocumentConstructionError> {
    let mut visitor = CstInterpreter::new(input);
    visitor.visit_root_handle(cst.root_handle(), cst)?;
    Ok(visitor.into_document_and_origin_map())
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
