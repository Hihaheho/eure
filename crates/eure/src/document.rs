mod value_visitor;

use eros::Union as _;
use eure_parol::parol_runtime::ParolError;
pub use eure_value::document::*;
use eure_value::identifier::IdentifierError;
use eure_value::string::EureStringError;
use eure_value::{document::constructor::PopError, path::PathSegment};

use crate::document::value_visitor::ValueVisitor;
use eure_tree::prelude::*;
use eure_tree::tree::InputSpan;
use thiserror::Error;

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
        error: EureStringError,
    },
    #[error("Invalid text binding at node {node_id:?}: {error}")]
    InvalidTextBinding {
        node_id: CstNodeId,
        error: EureStringError,
    },
    #[error("Invalid key type at node {node_id:?}")]
    InvalidKeyType { node_id: CstNodeId },
    #[error("Failed to pop path: {0}")]
    PopPath(#[from] PopError),
    #[error("Failed to parse tuple index: {value}")]
    InvalidTupleIndex { node_id: CstNodeId, value: String },
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
            DocumentConstructionError::InvalidTextBinding { node_id, .. } => {
                get_node_span(cst, *node_id)
            }
            DocumentConstructionError::InvalidKeyType { node_id } => get_node_span(cst, *node_id),
            _ => None,
        }
    }
}

/// Extract the InputSpan from a CST node if it has one
fn get_node_span(cst: &Cst, node_id: CstNodeId) -> Option<InputSpan> {
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
) -> eros::UResult<EureDocument, (ParolError, DocumentConstructionError)> {
    let tree = eure_parol::parse(input).union()?;
    let document = cst_to_document(input, &tree).union()?;
    Ok(document)
}

pub fn cst_to_document(input: &str, cst: &Cst) -> Result<EureDocument, DocumentConstructionError> {
    let mut visitor = ValueVisitor::new(input);
    visitor.visit_root_handle(cst.root_handle(), cst)?;
    Ok(visitor.into_document())
}
