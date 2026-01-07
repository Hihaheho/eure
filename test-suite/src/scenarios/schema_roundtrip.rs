use eure::query::{FormatSchema, TextFile};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct SchemaRoundtripScenario {
    pub schema: TextFile,
}

impl Scenario for SchemaRoundtripScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Single query call - chain: format_schema → schema_to_source → document_to_schema_query
        let actual = db.query(FormatSchema::new(self.schema.clone()))?;

        // Read original schema source
        let expected = db.asset(self.schema.clone())?.suspend()?;

        // Compare roundtrip result with original
        if *actual != expected.get() {
            return Err(ScenarioError::SchemaRoundtripMismatch {
                expected: expected.get().to_string(),
                actual: actual.as_ref().clone(),
            });
        }
        Ok(())
    }
}
