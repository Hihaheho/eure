use eure::query::{DiagnosticMessage, DiagnosticSeverity, GetDiagnostics, TextFile};
use query_flow::Db;

use crate::parser::DiagnosticItem;
use crate::scenarios::{Scenario, ScenarioError};

/// Resolved span positions from either explicit start/end or span string lookup.
struct ResolvedSpan {
    start: Option<i64>,
    end: Option<i64>,
}

/// Find a span string in editor content and return its byte offsets.
/// Returns an error if the string is not found or found multiple times.
fn resolve_span_string(
    editor_content: &str,
    span: &str,
    diagnostic_index: usize,
) -> Result<(i64, i64), ScenarioError> {
    let matches: Vec<_> = editor_content.match_indices(span).collect();

    match matches.len() {
        0 => Err(ScenarioError::SpanStringNotFound {
            diagnostic_index,
            span: span.to_string(),
        }),
        1 => {
            let start = matches[0].0 as i64;
            let end = start + span.len() as i64;
            Ok((start, end))
        }
        n => Err(ScenarioError::SpanStringAmbiguous {
            diagnostic_index,
            span: span.to_string(),
            occurrences: n,
        }),
    }
}

/// Resolve span positions for a diagnostic item.
/// Priority: span string > explicit start/end
fn resolve_span(
    expected: &DiagnosticItem,
    editor_content: &str,
    diagnostic_index: usize,
) -> Result<ResolvedSpan, ScenarioError> {
    if let Some(span) = &expected.span {
        let (start, end) = resolve_span_string(editor_content, span, diagnostic_index)?;
        Ok(ResolvedSpan {
            start: Some(start),
            end: Some(end),
        })
    } else {
        Ok(ResolvedSpan {
            start: expected.start,
            end: expected.end,
        })
    }
}

/// Diagnostics test scenario
#[derive(Debug, Clone)]
pub struct DiagnosticsScenario {
    /// Editor content with cursor position marked as `|_|`
    pub editor: TextFile,
    /// Optional schema file for validation (registered as "./schema.eure")
    pub schema: Option<TextFile>,
    /// Expected diagnostics (exact match, empty = no diagnostics expected)
    pub diagnostics: Vec<DiagnosticItem>,
}

impl Scenario for DiagnosticsScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual = db.query(GetDiagnostics::new(self.editor.clone()))?;

        let expected_strs: Vec<String> = self
            .diagnostics
            .iter()
            .map(format_expected_diagnostic)
            .collect();
        let actual_strs: Vec<String> = actual.iter().map(format_actual_diagnostic).collect();

        // Normalize trailing whitespace on each line for comparison
        let normalize = |s: &str| -> String {
            s.lines()
                .map(|line| line.trim_end())
                .collect::<Vec<_>>()
                .join("\n")
        };

        let expected_normalized: Vec<String> = expected_strs.iter().map(|s| normalize(s)).collect();
        let actual_normalized: Vec<String> = actual_strs.iter().map(|s| normalize(s)).collect();

        if expected_normalized != actual_normalized {
            return Err(ScenarioError::DiagnosticsMismatch {
                expected: expected_strs,
                actual: actual_strs,
            });
        }

        // Verify span positions if specified (span string or start/end)
        verify_span_positions(db, &self.diagnostics, &actual)?;

        Ok(())
    }
}

/// Verify that diagnostic spans match expected positions.
/// Supports both span string (resolved to offsets) and explicit start/end.
/// Uses the actual diagnostic's file to get the correct content for span lookup.
fn verify_span_positions(
    db: &impl Db,
    expected: &[DiagnosticItem],
    actual: &[DiagnosticMessage],
) -> Result<(), ScenarioError> {
    for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
        // Get the content of the file where the diagnostic is located
        let file_content = db.asset(act.file.clone())?;

        // Resolve span positions (from span string or explicit start/end)
        let resolved = resolve_span(exp, file_content.get(), i)?;

        // Check start position if specified
        if let Some(expected_start) = resolved.start {
            let actual_start = act.start as i64;
            if actual_start != expected_start {
                return Err(ScenarioError::SpanMismatch {
                    diagnostic_index: i,
                    field: "start".to_string(),
                    expected: expected_start,
                    actual: actual_start,
                });
            }
        }

        // Check end position if specified
        if let Some(expected_end) = resolved.end {
            let actual_end = act.end as i64;
            if actual_end != expected_end {
                return Err(ScenarioError::SpanMismatch {
                    diagnostic_index: i,
                    field: "end".to_string(),
                    expected: expected_end,
                    actual: actual_end,
                });
            }
        }
    }
    Ok(())
}

fn format_expected_diagnostic(item: &DiagnosticItem) -> String {
    let severity = item.severity.as_deref().unwrap_or("error");
    let message = item.message.as_deref().unwrap_or("");
    let source = item.source.as_deref().unwrap_or("");
    if source.is_empty() {
        format!("[{}] {}", severity, message)
    } else {
        format!("[{}] {} ({})", severity, message, source)
    }
}

fn format_actual_diagnostic(diag: &DiagnosticMessage) -> String {
    let severity = match diag.severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Hint => "hint",
    };
    let source = diag.file.to_string();
    format!("[{}] {} ({})", severity, diag.message, source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_span_string_found_once() {
        let content = "name = 123\nvalue = 456";
        let (start, end) = resolve_span_string(content, "123", 0).unwrap();
        assert_eq!(start, 7);
        assert_eq!(end, 10);
    }

    #[test]
    fn resolve_span_string_at_start() {
        let content = "hello world";
        let (start, end) = resolve_span_string(content, "hello", 0).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 5);
    }

    #[test]
    fn resolve_span_string_at_end() {
        let content = "hello world";
        let (start, end) = resolve_span_string(content, "world", 0).unwrap();
        assert_eq!(start, 6);
        assert_eq!(end, 11);
    }

    #[test]
    fn resolve_span_string_not_found() {
        let content = "name = 123";
        let result = resolve_span_string(content, "xyz", 0);
        assert!(matches!(
            result,
            Err(ScenarioError::SpanStringNotFound {
                diagnostic_index: 0,
                span,
            }) if span == "xyz"
        ));
    }

    #[test]
    fn resolve_span_string_ambiguous() {
        let content = "foo = 1\nfoo = 2";
        let result = resolve_span_string(content, "foo", 0);
        assert!(matches!(
            result,
            Err(ScenarioError::SpanStringAmbiguous {
                diagnostic_index: 0,
                span,
                occurrences: 2,
            }) if span == "foo"
        ));
    }

    #[test]
    fn resolve_span_string_with_quotes() {
        let content = r#"value = "invalid type""#;
        let (start, end) = resolve_span_string(content, "\"invalid type\"", 0).unwrap();
        assert_eq!(start, 8);
        assert_eq!(end, 22);
    }
}
