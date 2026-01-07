use eure::query::{FormatSchema, TextFile};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

/// Canonical schema scenario - tests that schema is in canonical format.
/// Triggered by `canonical_schema = true`.
/// Tests: format(schema) == schema (exact string match)
pub struct CanonicalSchemaScenario {
    pub schema: TextFile,
}

impl Scenario for CanonicalSchemaScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Get formatted schema
        let formatted = db.query(FormatSchema::new(self.schema.clone()))?;

        // Get original schema source
        let original = db.asset(self.schema.clone())?.suspend()?;

        // String comparison - formatted output must match exactly
        if *formatted != original.get() {
            return Err(ScenarioError::CanonicalSchemaMismatch {
                expected: original.get().to_string(),
                actual: formatted.as_ref().clone(),
            });
        }
        Ok(())
    }
}
