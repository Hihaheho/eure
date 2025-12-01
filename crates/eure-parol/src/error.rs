//! Parse error types for Eure.

use eure_tree::tree::InputSpan;
use parol_runtime::ParolError;

/// A parse error with extracted span and message information.
#[derive(Debug, Clone)]
pub struct EureParseError {
    /// The span in the input where the error occurred, if available.
    pub span: Option<InputSpan>,
    /// The error message.
    pub message: String,
    /// The kind of parse error.
    pub kind: ParseErrorKind,
}

/// The kind of parse error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// A syntax error (unexpected token, etc.)
    SyntaxError,
    /// A lexer error (invalid character, etc.)
    LexerError,
    /// Other error types
    Other,
}

impl EureParseError {
    /// Convert from a ParolError, extracting span info from its Debug representation.
    pub fn from_parol(error: &ParolError) -> Self {
        let debug_str = format!("{:?}", error);

        // Determine the kind based on the error variant
        let kind = match error {
            ParolError::ParserError(_) => ParseErrorKind::SyntaxError,
            ParolError::LexerError(_) => ParseErrorKind::LexerError,
            ParolError::UserError(_) => ParseErrorKind::Other,
        };

        // Try to extract the cause message
        let message = if let Some(cause_start) = debug_str.find("cause: \"") {
            if let Some(cause_end) = debug_str[cause_start + 8..].find('"') {
                debug_str[cause_start + 8..cause_start + 8 + cause_end]
                    .replace("\\n", "\n")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string()
            } else {
                format!("{}", error)
            }
        } else {
            format!("{}", error)
        };

        // Try to extract span from error_location
        let span = extract_span_from_debug(&debug_str);

        Self {
            span,
            message,
            kind,
        }
    }
}

/// Extract span information from the Debug representation of ParolError.
fn extract_span_from_debug(debug_str: &str) -> Option<InputSpan> {
    let loc_start = debug_str.find("error_location: Location {")?;
    let loc_end = debug_str[loc_start..].find('}')?;
    let loc_str = &debug_str[loc_start..loc_start + loc_end + 1];

    // Extract start position
    let start = loc_str
        .find("start: ")
        .and_then(|i| {
            let rest = &loc_str[i + 7..];
            rest.find(',').map(|j| rest[..j].parse::<u32>().ok())
        })
        .flatten()?;

    // Extract end position
    let end = loc_str
        .find("end: ")
        .and_then(|i| {
            let rest = &loc_str[i + 5..];
            rest.find(',').map(|j| rest[..j].parse::<u32>().ok())
        })
        .flatten()?;

    Some(InputSpan { start, end })
}

impl std::fmt::Display for EureParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EureParseError {}

impl From<ParolError> for EureParseError {
    fn from(error: ParolError) -> Self {
        Self::from_parol(&error)
    }
}

impl From<&ParolError> for EureParseError {
    fn from(error: &ParolError) -> Self {
        Self::from_parol(error)
    }
}
