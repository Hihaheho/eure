use eure::query::{CollectDiagnosticTargets, TextFile};
use eure_ls::queries::LspFileDiagnostics;
use query_flow::Db;

use crate::parser::DiagnosticItem;
use crate::scenarios::{Scenario, ScenarioError};

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
        // OpenDocuments asset is set up in Case::resolve_assets()
        // Collect all files needing diagnostics (editor + schema files)
        let all_files = db.query(CollectDiagnosticTargets::new())?;

        // Poll LspFileDiagnostics for each file (same as LSP server)
        let mut actual_diagnostics_by_file: Vec<(TextFile, lsp_types::Diagnostic)> = Vec::new();

        for file in all_files.iter() {
            let diagnostics = db.query(LspFileDiagnostics::new(file.clone()))?;
            for diag in diagnostics.iter() {
                actual_diagnostics_by_file.push((file.clone(), diag.clone()));
            }
        }

        // Format diagnostics for comparison
        let expected_strs: Vec<String> = self
            .diagnostics
            .iter()
            .map(format_expected_diagnostic)
            .collect();

        let actual_strs: Vec<String> = actual_diagnostics_by_file
            .iter()
            .map(|(file, diag)| format_lsp_diagnostic(diag, file))
            .collect();

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

        // Verify spans when specified.
        // Keep the order aligned with diagnostics comparison above.
        for (index, expected) in self.diagnostics.iter().enumerate() {
            let Some(expected_span) = expected.span.as_deref() else {
                continue;
            };

            let Some((file, actual)) = actual_diagnostics_by_file.get(index) else {
                continue;
            };

            let source: std::sync::Arc<eure::query::TextFileContent> = db.asset(file.clone())?;
            let (actual_start, actual_end) = diagnostic_to_offsets(actual, source.get());

            let (span_start, span_end) =
                find_span(source.get(), expected_span, expected.span_index, index)?;
            if actual_start as i64 != span_start as i64 {
                return Err(ScenarioError::SpanMismatch {
                    diagnostic_index: index,
                    field: "start".to_string(),
                    expected: span_start as i64,
                    actual: actual_start as i64,
                });
            }
            if actual_end as i64 != span_end as i64 {
                return Err(ScenarioError::SpanMismatch {
                    diagnostic_index: index,
                    field: "end".to_string(),
                    expected: span_end as i64,
                    actual: actual_end as i64,
                });
            }
        }

        Ok(())
    }
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

/// Format LSP diagnostic for comparison.
fn format_lsp_diagnostic(diag: &lsp_types::Diagnostic, file: &TextFile) -> String {
    let severity = match diag.severity {
        Some(lsp_types::DiagnosticSeverity::ERROR) => "error",
        Some(lsp_types::DiagnosticSeverity::WARNING) => "warning",
        Some(lsp_types::DiagnosticSeverity::INFORMATION) => "info",
        Some(lsp_types::DiagnosticSeverity::HINT) => "hint",
        _ => "error",
    };
    format!("[{}] {} ({})", severity, diag.message, file)
}

fn diagnostic_to_offsets(diag: &lsp_types::Diagnostic, source: &str) -> (usize, usize) {
    let line_offsets = compute_line_offsets(source);
    let start = position_to_offset(diag.range.start, source, &line_offsets);
    let end = position_to_offset(diag.range.end, source, &line_offsets);
    (start, end)
}

fn compute_line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, c) in source.char_indices() {
        if c == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

fn position_to_offset(pos: lsp_types::Position, source: &str, line_offsets: &[usize]) -> usize {
    let line = (pos.line as usize).min(line_offsets.len().saturating_sub(1));
    let line_start = line_offsets[line];
    let line_end = line_offsets.get(line + 1).copied().unwrap_or(source.len());
    let line_str = &source[line_start..line_end];

    let mut utf16_units = 0u32;
    let mut byte_offset = 0usize;
    for ch in line_str.chars() {
        if utf16_units >= pos.character {
            break;
        }
        utf16_units += ch.len_utf16() as u32;
        byte_offset += ch.len_utf8();
    }

    (line_start + byte_offset).min(source.len())
}

fn find_span(
    source: &str,
    span: &str,
    span_index: Option<i64>,
    diagnostic_index: usize,
) -> Result<(usize, usize), ScenarioError> {
    let matches: Vec<(usize, &str)> = source.match_indices(span).collect();
    if matches.is_empty() {
        return Err(ScenarioError::SpanStringNotFound {
            diagnostic_index,
            span: span.to_string(),
        });
    }

    if let Some(index) = span_index {
        let one_based = index.max(1) as usize;
        if let Some((start, _)) = matches.get(one_based - 1) {
            return Ok((*start, *start + span.len()));
        }
        return Err(ScenarioError::SpanStringNotFound {
            diagnostic_index,
            span: span.to_string(),
        });
    }

    if matches.len() > 1 {
        return Err(ScenarioError::SpanStringAmbiguous {
            diagnostic_index,
            span: span.to_string(),
            occurrences: matches.len(),
        });
    }

    let (start, _) = matches[0];
    Ok((start, start + span.len()))
}
