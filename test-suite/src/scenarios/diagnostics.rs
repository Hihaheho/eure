use eure::query::{DiagnosticMessage, DiagnosticSeverity, GetDiagnostics, TextFile};
use query_flow::Db;

use crate::parser::DiagnosticItem;
use crate::scenarios::{Scenario, ScenarioError};

/// Diagnostics test scenario
#[derive(Debug, Clone)]
pub struct DiagnosticsScenario {
    /// Editor content with cursor position marked as `|_|`
    pub editor: TextFile,
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
        Ok(())
    }
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
