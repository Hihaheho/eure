use crate::nodes::*;
use eure_value::{
    document::{EureDocument, InsertErrorKind},
    value::PathSegment,
};
use thiserror::Error;

use crate::{CstConstructError, tree::CstFacade, visitor::CstVisitor};

#[derive(Debug, Error)]
pub enum ValueVisitorError {
    #[error(transparent)]
    CstError(#[from] CstConstructError),
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("Failed to parse integer: {0}")]
    InvalidInteger(String),
    #[error("Failed to parse float: {0}")]
    InvalidFloat(String),
    #[error("Document insert error: {0}")]
    DocumentInsert(#[from] InsertErrorKind),
}

pub struct ValueVisitor<'a> {
    input: &'a str,
    // Main document being built
    document: EureDocument,
    // Stack of paths for nested sections
    path_stack: Vec<Vec<PathSegment>>,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            document: EureDocument::new(),
            path_stack: vec![vec![]], // Start with empty root path
        }
    }

    pub fn into_document(self) -> EureDocument {
        self.document
    }

    /// Get the current base path from the path stack
    fn current_path(&self) -> Option<&[PathSegment]> {
        self.path_stack.last().map(|path| path.as_slice())
    }
}

impl<F: CstFacade> CstVisitor<F> for ValueVisitor<'_> {
    type Error = ValueVisitorError;
}
