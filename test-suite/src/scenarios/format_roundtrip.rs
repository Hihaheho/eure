use eure::query::{FormatSchema, TextFile};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

/// Format roundtrip scenario - compares source strings.
/// Tests that: input == format(schema(input))
pub struct FormatRoundtripScenario {
    pub schema: TextFile,
}

impl Scenario for FormatRoundtripScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Get formatted schema
        let formatted = db.query(FormatSchema::new(self.schema.clone()))?;

        // Get original schema source
        let original = db.asset(self.schema.clone())?.suspend()?;

        // Compare the strings
        if original.get() != formatted.as_ref() {
            return Err(ScenarioError::FormatRoundtripMismatch {
                original: original.get().to_string(),
                formatted: formatted.as_ref().clone(),
            });
        }
        Ok(())
    }
}
