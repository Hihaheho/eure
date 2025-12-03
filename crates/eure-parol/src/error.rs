//! Parse error types for Eure.
use std::fmt;

use eure_tree::tree::InputSpan;
use parol_runtime::{LexerError, ParolError, ParserError, SyntaxError};

/// A parse error with extracted span and message information.
#[derive(Debug, Clone)]
pub struct EureParseError {
    pub entries: Vec<ParseErrorEntry>,
}

impl EureParseError {
    pub fn new_entry(span: Option<InputSpan>, message: String, kind: ParseErrorKind) -> Self {
        Self {
            entries: vec![ParseErrorEntry {
                span,
                message,
                kind,
                source: vec![],
            }],
        }
    }
}

impl fmt::Display for EureParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", entry)?;
        }
        Ok(())
    }
}

impl fmt::Display for ParseErrorEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(span) = &self.span {
            write!(f, " at {}..{}", span.start, span.end)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseErrorEntry {
    /// The span in the input where the error occurred, if available.
    pub span: Option<InputSpan>,
    /// The error message.
    pub message: String,
    /// The kind of parse error.
    pub kind: ParseErrorKind,
    pub source: Vec<ParseErrorEntry>,
}

/// The kind of parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// A syntax error (unexpected token, etc.)
    SyntaxError {
        unexpected_tokens: Vec<UnexpectedToken>,
        expected_tokens: Vec<String>,
    },
    UnprocessedInput,
    /// A lexer error (invalid character, etc.)
    LexerError,
    /// Other error types
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnexpectedToken {
    pub name: String,
    pub token_type: String,
    pub token: InputSpan,
}

impl From<parol_runtime::UnexpectedToken> for UnexpectedToken {
    fn from(
        parol_runtime::UnexpectedToken {
            name,
            token_type,
            token,
        }: parol_runtime::UnexpectedToken,
    ) -> Self {
        Self {
            name,
            token_type,
            token: InputSpan {
                start: token.start,
                end: token.end,
            },
        }
    }
}

impl From<ParolError> for EureParseError {
    fn from(error: ParolError) -> Self {
        match error {
            ParolError::ParserError(parser_error) => parser_error.into(),
            ParolError::LexerError(lexer_error) => lexer_error.into(),
            ParolError::UserError(error) => error.into(),
        }
    }
}

impl From<ParserError> for EureParseError {
    fn from(error: ParserError) -> Self {
        match error {
            ParserError::TreeError { source } => source.into(),
            ParserError::DataError(error) => {
                EureParseError::new_entry(None, error.to_string(), ParseErrorKind::Other)
            }
            ParserError::PredictionError { cause } => {
                EureParseError::new_entry(None, cause, ParseErrorKind::Other)
            }
            ParserError::SyntaxErrors { entries } => EureParseError {
                entries: entries.into_iter().map(ParseErrorEntry::from).collect(),
            },
            ParserError::UnprocessedInput {
                input: _,
                last_token,
            } => EureParseError::new_entry(
                Some(InputSpan {
                    start: last_token.start,
                    end: last_token.end,
                }),
                "Unprocessed input".to_string(),
                ParseErrorKind::UnprocessedInput,
            ),
            ParserError::Unsupported {
                context,
                error_location,
            } => EureParseError::new_entry(
                Some(InputSpan {
                    start: error_location.start,
                    end: error_location.end,
                }),
                format!("Unsupported: {context}"),
                ParseErrorKind::Other,
            ),
            ParserError::TooManyErrors { count } => EureParseError::new_entry(
                None,
                format!("Too many errors: {count}"),
                ParseErrorKind::Other,
            ),
            ParserError::RecoveryFailed => EureParseError::new_entry(
                None,
                "Recovery failed".to_string(),
                ParseErrorKind::Other,
            ),
            ParserError::InternalError(error) => EureParseError::new_entry(
                None,
                format!("Internal error: {error}"),
                ParseErrorKind::Other,
            ),
        }
    }
}

impl From<parol_runtime::syntree::Error> for EureParseError {
    fn from(error: parol_runtime::syntree::Error) -> Self {
        EureParseError::new_entry(None, error.to_string(), ParseErrorKind::Other)
    }
}

impl From<parol_runtime::SyntaxError> for ParseErrorEntry {
    fn from(error: parol_runtime::SyntaxError) -> Self {
        let SyntaxError {
            cause,
            input: _,
            error_location,
            unexpected_tokens,
            expected_tokens,
            source,
        } = error;
        ParseErrorEntry {
            span: Some(InputSpan {
                start: error_location.start,
                end: error_location.end,
            }),
            message: cause,
            kind: ParseErrorKind::SyntaxError {
                unexpected_tokens: unexpected_tokens
                    .into_iter()
                    .map(UnexpectedToken::from)
                    .collect(),
                expected_tokens: expected_tokens.iter().cloned().collect(),
            },
            source: source
                .map(|source| EureParseError::from(*source).entries)
                .unwrap_or_default(),
        }
    }
}

impl From<LexerError> for EureParseError {
    fn from(error: LexerError) -> Self {
        match error {
            LexerError::TokenBufferEmptyError => EureParseError::new_entry(
                None,
                "Token buffer empty".to_string(),
                ParseErrorKind::LexerError,
            ),
            LexerError::InternalError(error) => EureParseError::new_entry(
                None,
                format!("Internal error: {error}"),
                ParseErrorKind::Other,
            ),
            LexerError::LookaheadExceedsMaximum => EureParseError::new_entry(
                None,
                "Lookahead exceeds maximum".to_string(),
                ParseErrorKind::LexerError,
            ),
            LexerError::LookaheadExceedsTokenBufferLength => EureParseError::new_entry(
                None,
                "Lookahead exceeds token buffer length".to_string(),
                ParseErrorKind::LexerError,
            ),
            LexerError::ScannerStackEmptyError => EureParseError::new_entry(
                None,
                "Scanner stack empty".to_string(),
                ParseErrorKind::LexerError,
            ),
            LexerError::RecoveryError(error) => EureParseError::new_entry(
                None,
                format!("Recovery error: {error}"),
                ParseErrorKind::LexerError,
            ),
        }
    }
}

impl From<anyhow::Error> for EureParseError {
    fn from(error: anyhow::Error) -> Self {
        EureParseError::new_entry(None, error.to_string(), ParseErrorKind::Other)
    }
}
