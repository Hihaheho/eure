use eure::query::{TextFile, get_hover};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

/// Hover test scenario
#[derive(Debug, Clone)]
pub struct HoverScenario {
    /// Editor content (cursor marker already stripped)
    pub editor: TextFile,
    /// Byte offset of the cursor in the editor content
    pub cursor: u32,
    /// Expected hover markdown; `None` expects no hover at the cursor
    pub hover: Option<String>,
}

impl Scenario for HoverScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual = get_hover(db, &self.editor, self.cursor)?.map(|hover| hover.contents);

        // Compare ignoring surrounding whitespace: fixtures are code blocks
        // that end with a newline.
        let expected_trimmed = self.hover.as_deref().map(str::trim);
        let actual_trimmed = actual.as_deref().map(str::trim);
        if expected_trimmed != actual_trimmed {
            return Err(ScenarioError::HoverMismatch {
                expected: self.hover,
                actual,
            });
        }

        Ok(())
    }
}
