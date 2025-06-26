use eure_parol::parol_runtime::{LexerError, ParolError, ParserError, SyntaxError};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Convert a ParolError to an LSP Diagnostic
pub fn error_to_diagnostic(error: &ParolError) -> Vec<Diagnostic> {
    // The default position is the beginning of the file
    let position = Position::new(0, 0);
    let range = Range::new(position, position);
    let mut message = error.to_string();

    match error {
        ParolError::ParserError(parser_error) => match parser_error {
            ParserError::TreeError { .. } => {}
            ParserError::DataError(_) => {}
            ParserError::PredictionError { .. } => {}
            ParserError::SyntaxErrors { entries } => {
                return entries.iter().map(parse_error_to_diagnostic).collect();
            }
            ParserError::UnprocessedInput { .. } => {}
            ParserError::PopOnEmptyScannerStateStack { .. } => {}
            ParserError::TooManyErrors { count } => {
                message = format!("Too many errors: {count}");
            }
            ParserError::RecoveryFailed => {}
            ParserError::InternalError(_) => {}
        },
        ParolError::LexerError(lexer_error) => match lexer_error {
            LexerError::TokenBufferEmptyError => {}
            LexerError::InternalError(_) => {}
            LexerError::LookaheadExceedsMaximum => {}
            LexerError::LookaheadExceedsTokenBufferLength => {}
            LexerError::ScannerStackEmptyError => {}
            LexerError::RecoveryError(_) => {}
            LexerError::RegexError(_) => {}
        },
        ParolError::UserError(_) => {}
    }

    // Create a diagnostic with the error message
    vec![Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("eure".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }]
}

/// Convert a SyntaxError to an LSP Diagnostic.
/// Parol line and column are 1-based, but LSP is 0-based.
pub fn parse_error_to_diagnostic(error: &SyntaxError) -> Diagnostic {
    let start = Position {
        line: error.error_location.start_line - 1,
        character: error.error_location.start_column - 1,
    };
    let end = Position {
        line: error.error_location.end_line - 1,
        character: error.error_location.end_column - 1,
    };
    let range = Range::new(start, end);
    let message = error.cause.clone();
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("eure".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}
