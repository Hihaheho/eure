//! Diagnostic collection queries.

use query_flow::query;

use crate::assets::TextFile;
use crate::config::ParseCst;
use crate::schema::{
    ErrorSpan, GetSchemaExtensionDiagnostics, ResolveSchema, ValidateAgainstSchema,
};
use eure_parol::EureParseError;

/// Severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// A diagnostic message with source location.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticMessage {
    /// Start byte offset in the source.
    pub start: usize,
    /// End byte offset in the source.
    pub end: usize,
    /// The diagnostic message.
    pub message: String,
    /// Severity of the diagnostic.
    pub severity: DiagnosticSeverity,
}

/// Collect all diagnostics for a document.
///
/// This includes:
/// - Parse errors (from tolerant parsing)
/// - Schema validation errors (if a schema is resolved)
///
/// Returns an empty vec if the file cannot be parsed.
#[query]
pub fn get_diagnostics(
    db: &impl Db,
    file: TextFile,
) -> Result<Vec<DiagnosticMessage>, query_flow::QueryError> {
    let mut diagnostics = Vec::new();

    // 1. Collect parse errors
    if let Some(parsed) = db.query(ParseCst::new(file.clone()))?.as_ref() {
        if let Some(error) = &parsed.error {
            diagnostics.extend(parse_error_to_diagnostics(error));
        }

        // 2. Collect $schema extension errors (e.g., wrong type)
        let schema_ext_errors = db.query(GetSchemaExtensionDiagnostics::new(file.clone()))?;
        diagnostics.extend(schema_ext_errors.iter().map(error_span_to_diagnostic));

        // 3. Collect schema validation errors (only if parsing succeeded)
        if parsed.error.is_none()
            && let Some(schema_file) = db.query(ResolveSchema::new(file.clone()))?.as_ref()
        {
            let validation_errors = db.query(ValidateAgainstSchema::new(
                file.clone(),
                schema_file.clone(),
            ))?;
            diagnostics.extend(validation_errors.iter().map(error_span_to_diagnostic));
        }
    }

    Ok(diagnostics)
}

/// Convert parse errors to diagnostic messages.
fn parse_error_to_diagnostics(error: &EureParseError) -> Vec<DiagnosticMessage> {
    error
        .entries
        .iter()
        .map(|entry| {
            let (start, end) = entry
                .span
                .map(|s| (s.start as usize, s.end as usize))
                .unwrap_or((0, 1));

            DiagnosticMessage {
                start,
                end,
                message: entry.message.clone(),
                severity: DiagnosticSeverity::Error,
            }
        })
        .collect()
}

/// Convert a schema validation error span to a diagnostic message.
fn error_span_to_diagnostic(span: &ErrorSpan) -> DiagnosticMessage {
    DiagnosticMessage {
        start: span.start,
        end: span.end,
        message: span.message.clone(),
        severity: DiagnosticSeverity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_severity_eq() {
        assert_eq!(DiagnosticSeverity::Error, DiagnosticSeverity::Error);
        assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Warning);
    }
}
