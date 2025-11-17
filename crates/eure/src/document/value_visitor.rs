use eure_tree::prelude::*;
use eure_value::{
    document::{EureDocument, InsertError, constructor::DocumentConstructor},
    path::PathSegment,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocumentConstructionError {
    #[error(transparent)]
    CstError(#[from] CstConstructError),
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("Failed to parse integer: {0}")]
    InvalidInteger(String),
    #[error("Failed to parse float: {0}")]
    InvalidFloat(String),
    #[error("Document insert error: {0}")]
    DocumentInsert(#[from] InsertError),
    #[error("Unprocessed segments: {segments:?}")]
    UnprocessedSegments { segments: Vec<PathSegment> },
}

pub struct ValueVisitor<'a> {
    input: &'a str,
    // Main document being built
    document: DocumentConstructor,
    segments: Vec<PathSegment>,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            document: DocumentConstructor::new(),
            segments: vec![],
        }
    }

    pub fn into_document(self) -> EureDocument {
        self.document.finish()
    }

    fn collect_path_segments(
        &mut self,
        visit: impl FnOnce(&mut Self) -> Result<(), DocumentConstructionError>,
    ) -> Result<Vec<PathSegment>, DocumentConstructionError> {
        if !self.segments.is_empty() {
            return Err(DocumentConstructionError::UnprocessedSegments {
                segments: self.segments.clone(),
            });
        }
        visit(self)?;
        let segments = std::mem::take(&mut self.segments);
        Ok(segments)
    }
}

impl<F: CstFacade> CstVisitor<F> for ValueVisitor<'_> {
    type Error = DocumentConstructionError;
}
