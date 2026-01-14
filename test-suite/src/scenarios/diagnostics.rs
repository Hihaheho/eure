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
