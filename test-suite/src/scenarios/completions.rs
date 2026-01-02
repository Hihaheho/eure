use eure::query::TextFile;
use query_flow::Db;

use crate::parser::CompletionItem;
use crate::scenarios::{Scenario, ScenarioError};

/// Completions test scenario
#[derive(Debug, Clone)]
pub struct CompletionsScenario {
    /// Editor content with cursor position marked as `|_|`
    pub editor: TextFile,
    /// Expected completions (exact match)
    pub completions: Vec<CompletionItem>,
    /// Trigger character (e.g., ".", "@", "=")
    pub trigger: Option<String>,
}

impl Scenario for CompletionsScenario {
    fn run(self, _db: &impl Db) -> Result<(), ScenarioError> {
        Err(ScenarioError::Unimplemented {
            scenario_name: "completions".to_string(),
        })
    }
}
