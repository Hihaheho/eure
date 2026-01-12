//! Diagnostic collection queries.

use eure_parol::EureParseError;
use query_flow::{Db, QueryError, query};

use crate::query::parse::ParseCst;
use crate::report::{ErrorReport, ErrorReports};

use super::assets::TextFile;
use super::schema::{GetSchemaExtensionDiagnostics, ValidateAgainstSchema};

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
    /// The file this diagnostic belongs to.
    pub file: TextFile,
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
pub fn get_diagnostics(db: &impl Db, file: TextFile) -> Result<Vec<DiagnosticMessage>, QueryError> {
    let mut diagnostics = Vec::new();

    // 1. Collect parse errors
    let parsed = db.query(ParseCst::new(file.clone()))?;
    if let Some(error) = &parsed.error {
        diagnostics.extend(parse_error_to_diagnostics(error, file.clone()));
    }

    // Schema-related diagnostics only run if parsing succeeded
    if parsed.error.is_none() {
        // 2. Collect $schema extension errors (e.g., wrong type)
        let schema_ext_errors = db.query(GetSchemaExtensionDiagnostics::new(file.clone()))?;
        diagnostics.extend(schema_ext_errors.iter().map(error_report_to_diagnostic));

        // 3. Collect schema validation errors (including schema file not found and conversion errors)
        // ValidateAgainstSchema resolves the schema internally and handles file not found with proper origin
        match db.query(ValidateAgainstSchema::new(file.clone())) {
            Ok(reports) => {
                diagnostics.extend(reports.iter().map(error_report_to_diagnostic));
            }
            Err(QueryError::UserError(e)) => {
                // Schema conversion errors are returned as UserError containing ErrorReports
                if let Some(reports) = e.downcast_ref::<ErrorReports>() {
                    diagnostics.extend(reports.iter().map(error_report_to_diagnostic));
                } else {
                    // Re-propagate unknown user errors
                    return Err(QueryError::UserError(e));
                }
            }
            Err(other) => return Err(other),
        }
    }

    Ok(diagnostics)
}

/// Convert parse errors to diagnostic messages.
fn parse_error_to_diagnostics(error: &EureParseError, file: TextFile) -> Vec<DiagnosticMessage> {
    error
        .entries
        .iter()
        .map(|entry| {
            // FIXME: Fallback to file start (0, 1) when span is missing.
            // This causes errors to be reported at the wrong location.
            // Should propagate the missing span information or use a better heuristic.
            let (start, end) = entry
                .span
                .map(|s| (s.start as usize, s.end as usize))
                .unwrap_or((0, 1));

            DiagnosticMessage {
                file: file.clone(),
                start,
                end,
                message: entry.message.clone(),
                severity: DiagnosticSeverity::Error,
            }
        })
        .collect()
}

/// Convert an error report to a diagnostic message.
fn error_report_to_diagnostic(report: &ErrorReport) -> DiagnosticMessage {
    DiagnosticMessage {
        file: report.primary_origin.file.clone(),
        start: report.primary_origin.span.start as usize,
        end: report.primary_origin.span.end as usize,
        message: report.title.to_string(),
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
