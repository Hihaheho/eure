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
        let editor_content = db.asset(self.editor)?.suspend()?;
        verify_span_positions(&self.diagnostics, &actual, editor_content.get())?;

        Ok(())
    }
}

/// Verify that diagnostic spans match expected positions.
/// Supports both span string (resolved to offsets) and explicit start/end.
fn verify_span_positions(
    expected: &[DiagnosticItem],
    actual: &[DiagnosticMessage],
    editor_content: &str,
) -> Result<(), ScenarioError> {
    for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
        // Resolve span positions (from span string or explicit start/end)
        let resolved = resolve_span(exp, editor_content, i)?;

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
    format!("[{}] {}", severity, message)
}

fn format_actual_diagnostic(diag: &DiagnosticMessage) -> String {
    let severity = match diag.severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Hint => "hint",
    };
    format!("[{}] {}", severity, diag.message)
}
