use eure_parol::EureParseError;

use crate::query::TextFile;

#[derive(Debug, thiserror::Error)]
pub enum EureQueryError {
    #[error("File not found: {0}")]
    FileNotFound(TextFile),
    #[error("Content not found: {0}")]
    ContentNotFound(TextFile),
    #[error("Parse error: {0}")]
    ParseError(EureParseError),
}
