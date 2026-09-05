use eure::query::{TextFile, get_completions};
use query_flow::Db;

use crate::parser::CompletionItem;
use crate::scenarios::{Scenario, ScenarioError};

/// Completions test scenario
#[derive(Debug, Clone)]
pub struct CompletionsScenario {
    /// Editor content (cursor marker already stripped)
    pub editor: TextFile,
    /// Byte offset of the cursor in the editor content
    pub cursor: u32,
    /// Expected completions, in order (exact match on label; kind when given)
    pub completions: Vec<CompletionItem>,
}

impl Scenario for CompletionsScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual = get_completions(db, &self.editor, self.cursor)?;

        let matches = actual.len() == self.completions.len()
            && actual
                .iter()
                .zip(&self.completions)
                .all(|(item, expected)| {
                    item.label == expected.label
                        && expected
                            .kind
                            .as_deref()
                            .is_none_or(|kind| kind == item.kind.as_str())
                });

        if !matches {
            return Err(ScenarioError::CompletionsMismatch {
                expected: self
                    .completions
                    .iter()
                    .map(|item| match &item.kind {
                        Some(kind) => format!("{} ({})", item.label, kind),
                        None => item.label.clone(),
                    })
                    .collect(),
                actual: actual
                    .iter()
                    .map(|item| format!("{} ({})", item.label, item.kind.as_str()))
                    .collect(),
            });
        }

        Ok(())
    }
}
