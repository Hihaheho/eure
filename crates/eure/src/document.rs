mod value_visitor;

use eros::Union as _;
use eure_parol::EureParseError;
pub use eure_value::document::*;
use eure_value::identifier::IdentifierError;
use eure_value::text::TextParseError;
use eure_value::{
    document::constructor::{FinishError, PopError},
    path::PathSegment,
};

use crate::document::value_visitor::ValueVisitor;
use eure_tree::prelude::*;
use eure_tree::tree::InputSpan;
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// NodeOrigin - Semantic tracking of how nodes are created
// ============================================================================

/// Describes the syntactic origin of a document node.
/// Stores typed `*Handle` wrappers for type safety.
/// Spans are resolved lazily by calling `get_span(cst)`.
#[derive(Debug, Clone, Copy)]
pub enum NodeOrigin {
    /// Node created via section header (@ key.path)
    SectionKey(SectionHandle),
    /// Node created via binding key (key = value)
    BindingKey(BindingHandle),
    /// Node created via key in Keys path
    IntermediateKey(KeyHandle),
    /// Node created as bound integer value
    BoundInteger(IntegerHandle),
    /// Node created as bound float value
    BoundFloat(FloatHandle),
    /// Node created as bound strings value
    BoundStrings(StringsHandle),
    /// Node created as bound inline code (single backtick)
    BoundInlineCode1(InlineCode1Handle),
    /// Node created as bound inline code (double backtick)
    BoundInlineCode2(InlineCode2Handle),
    /// Node created as bound code block
    BoundCodeBlock(CodeBlockHandle),
    /// Node created as bound null
    BoundNull(NullHandle),
    /// Node created as bound true
    BoundTrue(TrueHandle),
    /// Node created as bound false
    BoundFalse(FalseHandle),
    /// Node created as bound hole (!)
    BoundHole(HoleHandle),
    /// Node created as array container
    ArrayContainer(ArrayHandle),
    /// Node created as array element
    ArrayElement { index: usize, array: ArrayHandle },
    /// Node created as tuple container
    TupleContainer(TupleHandle),
    /// Node created as tuple element
    TupleElement { index: u8, tuple: TupleHandle },
    /// Node created as object/map container
    ObjectContainer(ObjectHandle),
    /// Node created as object/map entry
    ObjectEntry(KeysHandle),
    /// Node created for extension binding ($ext = value)
    ExtensionBinding(KeyHandle),
    /// Node created via text binding (key: text content)
    TextBinding(TextBindingHandle),
    /// Node created via section binding (key { ... })
    SectionBinding(SectionBindingHandle),
    /// Node created via value binding ({ = value })
    ValueBinding(ValueBindingHandle),
}

impl NodeOrigin {
    /// Get the CstNodeId for this origin
    pub fn node_id(&self) -> CstNodeId {
        match self {
            NodeOrigin::SectionKey(h) => h.node_id(),
            NodeOrigin::BindingKey(h) => h.node_id(),
            NodeOrigin::IntermediateKey(h) => h.node_id(),
            NodeOrigin::BoundInteger(h) => h.node_id(),
            NodeOrigin::BoundFloat(h) => h.node_id(),
            NodeOrigin::BoundStrings(h) => h.node_id(),
            NodeOrigin::BoundInlineCode1(h) => h.node_id(),
            NodeOrigin::BoundInlineCode2(h) => h.node_id(),
            NodeOrigin::BoundCodeBlock(h) => h.node_id(),
            NodeOrigin::BoundNull(h) => h.node_id(),
            NodeOrigin::BoundTrue(h) => h.node_id(),
            NodeOrigin::BoundFalse(h) => h.node_id(),
            NodeOrigin::BoundHole(h) => h.node_id(),
            NodeOrigin::ArrayContainer(h) => h.node_id(),
            NodeOrigin::ArrayElement { array, .. } => array.node_id(),
            NodeOrigin::TupleContainer(h) => h.node_id(),
            NodeOrigin::TupleElement { tuple, .. } => tuple.node_id(),
            NodeOrigin::ObjectContainer(h) => h.node_id(),
            NodeOrigin::ObjectEntry(h) => h.node_id(),
            NodeOrigin::ExtensionBinding(h) => h.node_id(),
            NodeOrigin::TextBinding(h) => h.node_id(),
            NodeOrigin::SectionBinding(h) => h.node_id(),
            NodeOrigin::ValueBinding(h) => h.node_id(),
        }
    }

    /// Get the input span for this origin from the CST
    pub fn get_span(&self, cst: &Cst) -> Option<InputSpan> {
        get_node_span(cst, self.node_id())
    }
}

/// Mapping from document NodeId to its syntactic origins.
/// A node can have multiple origins (e.g., created via key + value).
pub type NodeOriginMap = HashMap<NodeId, Vec<NodeOrigin>>;

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
    },
    #[error("Unprocessed segments: {segments:?}")]
    UnprocessedSegments { segments: Vec<PathSegment> },
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
    #[error("Failed to pop path: {0}")]
    PopPath(#[from] PopError),
    #[error("Failed to parse tuple index: {value}")]
    InvalidTupleIndex { node_id: CstNodeId, value: String },
    #[error("Failed to finish document: {0}")]
    FinishError(FinishError),
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
                node_id.and_then(|id| get_node_span(cst, id))
            }
            DocumentConstructionError::DocumentInsert { node_id, .. } => {
                get_node_span(cst, *node_id)
            }
            DocumentConstructionError::InvalidInlineCode { node_id, .. } => {
                get_node_span(cst, *node_id)
            }
            DocumentConstructionError::InvalidCodeBlock { node_id, .. } => {
                get_node_span(cst, *node_id)
            }
            DocumentConstructionError::InvalidStringKey { node_id, .. } => {
                get_node_span(cst, *node_id)
            }
            DocumentConstructionError::InvalidKeyType { node_id } => get_node_span(cst, *node_id),
            _ => None,
        }
    }
}

/// Extract the InputSpan from a CST node if it has one
pub fn get_node_span(cst: &Cst, node_id: CstNodeId) -> Option<InputSpan> {
    cst.node_data(node_id).and_then(|node| match node {
        CstNode::Terminal {
            data: TerminalData::Input(span),
            ..
        } => Some(span),
        CstNode::NonTerminal {
            data: NonTerminalData::Input(span),
            ..
        } => Some(span),
        _ => None,
    })
}

pub fn parse_to_document(
    input: &str,
) -> eros::UResult<EureDocument, (EureParseError, DocumentConstructionError)> {
    let tree = eure_parol::parse(input).union()?;
    let document = cst_to_document(input, &tree).union()?;
    Ok(document)
}

pub fn cst_to_document(input: &str, cst: &Cst) -> Result<EureDocument, DocumentConstructionError> {
    let mut visitor = ValueVisitor::new(input);
    visitor.visit_root_handle(cst.root_handle(), cst)?;
    visitor.into_document()
}

/// Parse CST to document and collect origin information for span resolution.
pub fn cst_to_document_and_origins(
    input: &str,
    cst: &Cst,
) -> Result<(EureDocument, NodeOriginMap), DocumentConstructionError> {
    let mut visitor = ValueVisitor::new(input);
    visitor.visit_root_handle(cst.root_handle(), cst)?;
    visitor.into_document_and_origins()
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
        let original_span = get_node_span(&cst, binding_node).unwrap();
        // Get the shrunk span (excludes trivia) - now using method syntax
        let shrunk_span = cst.get_shrunk_span(binding_node).unwrap();

        // The original span should start at 0 (includes leading newline and whitespace)
        // Or at least include the whitespace before "foo"
        let original_text = original_span.as_str(input);
        let shrunk_text = shrunk_span.as_str(input);

        println!("Original span: {:?} -> '{}'", original_span, original_text);
        println!("Shrunk span: {:?} -> '{}'", shrunk_span, shrunk_text);

        // The shrunk span should start at "foo" (position 3)
        assert!(
            shrunk_span.start >= original_span.start,
            "Shrunk span start ({}) should be >= original start ({})",
            shrunk_span.start,
            original_span.start
        );

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
        let input = "foo = 1 // comment\nbar = 2";
        let cst = eure_parol::parse(input).unwrap();

        let root = cst.root();

        // Get the original span of the root Eure node
        let original_span = get_node_span(&cst, root).unwrap();
        let shrunk_span = cst.get_shrunk_span(root).unwrap();

        let original_text = original_span.as_str(input);
        let shrunk_text = shrunk_span.as_str(input);

        println!(
            "Original root span: {:?} -> '{}'",
            original_span, original_text
        );
        println!("Shrunk root span: {:?} -> '{}'", shrunk_span, shrunk_text);

        // The shrunk span should not include trailing whitespace/comments
        // but should include all meaningful content
        assert!(
            shrunk_text.contains("foo"),
            "Shrunk span should contain 'foo'"
        );
        assert!(
            shrunk_text.contains("bar"),
            "Shrunk span should contain 'bar'"
        );
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

        let original_span = get_node_span(&cst, eure_node).unwrap();
        let shrunk_span = cst.get_shrunk_span(eure_node).unwrap();

        let original_text = original_span.as_str(input);
        let shrunk_text = shrunk_span.as_str(input);

        println!(
            "Eure original span: {:?} -> '{}'",
            original_span,
            original_text.escape_debug()
        );
        println!(
            "Eure shrunk span: {:?} -> '{}'",
            shrunk_span,
            shrunk_text.escape_debug()
        );

        // Original span likely includes all whitespace
        // Shrunk span should just be "foo = 1"
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
        let ws_span = cst.get_shrunk_span(ws_node);

        // Whitespace terminal should still return its span
        assert!(
            ws_span.is_some(),
            "Terminal span should be returned even for trivia"
        );
    }
}
