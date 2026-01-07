use eure::query::{FormatSchema, TextFile};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct FormatSchemaScenario {
    pub schema: TextFile,
    pub formatted_schema: TextFile,
}

impl Scenario for FormatSchemaScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Single query call - chain: format_schema → schema_to_source → document_to_schema_query
        let actual = db.query(FormatSchema::new(self.schema.clone()))?;

        // Read expected formatted schema
        let expected = db.asset(self.formatted_schema.clone())?.suspend()?;

        // Compare formatted result with expected
        if *actual != expected.get() {
            return Err(ScenarioError::FormatSchemaMismatch {
                expected: expected.get().to_string(),
                actual: actual.as_ref().clone(),
            });
        }
        Ok(())
    }
}
