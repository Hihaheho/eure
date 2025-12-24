//! Error types for eure-mark

use thiserror::Error;

/// Errors that can occur during eumd document processing
#[derive(Debug, Error)]
pub enum EumdError {
    /// Parse error from eure-parol
    #[error("Parse error: {0}")]
    Parse(String),

    /// Document parsing error
    #[error("Document error: {0}")]
    Document(#[from] eure_document::parse::ParseError),

    /// Reference check errors
    #[error("Reference errors:\n{}", format_reference_errors(.0))]
    ReferenceErrors(Vec<ReferenceError>),
}

/// A single reference error
#[derive(Debug, Clone)]
pub struct ReferenceError {
    /// Type of reference
    pub ref_type: ReferenceType,
    /// The key that was referenced
    pub key: String,
    /// Location description (e.g., "in section 'intro'")
    pub location: String,
}

/// Type of reference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceType {
    /// Citation reference: !cite[key]
    Cite,
    /// Footnote reference: !footnote[key]
    Footnote,
    /// Section reference: !ref[key]
    Section,
}

impl std::fmt::Display for ReferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferenceType::Cite => write!(f, "cite"),
            ReferenceType::Footnote => write!(f, "footnote"),
            ReferenceType::Section => write!(f, "ref"),
        }
    }
}

impl std::fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Undefined {}[{}] {}",
            self.ref_type, self.key, self.location
        )
    }
}

fn format_reference_errors(errors: &[ReferenceError]) -> String {
    errors
        .iter()
        .map(|e| format!("  - {e}"))
        .collect::<Vec<_>>()
        .join("\n")
}
