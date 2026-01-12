use std::sync::Arc;

use eure::query::TextFile;
use eure_json_schema::EureSchemaToJsonSchemaQuery;
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct EureSchemaToJsonSchemaScenario {
    pub schema: TextFile,
    pub output_json_schema: TextFile,
}

impl Scenario for EureSchemaToJsonSchemaScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual: Arc<serde_json::Value> =
            db.query(EureSchemaToJsonSchemaQuery::new(self.schema.clone()))?;

        // Read expected JSON schema
        let expected_str = db.asset(self.output_json_schema.clone())?;

        let expected: serde_json::Value =
            serde_json::from_str(expected_str.get()).map_err(|e| {
                ScenarioError::JsonParseError {
                    message: e.to_string(),
                }
            })?;

        if *actual != expected {
            Err(ScenarioError::JsonSchemaMismatch {
                expected: Box::new(expected),
                actual: Box::new(actual.as_ref().clone()),
            })
        } else {
            Ok(())
        }
    }
}
