use eure::query::{TextFile, ValidateAgainstExplicitSchema};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct SchemaValidationScenario {
    pub input: TextFile,
    pub schema: TextFile,
}

impl Scenario for SchemaValidationScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let validation = db.query(ValidateAgainstExplicitSchema::new(
            self.input.clone(),
            self.schema.clone(),
        ))?;

        if validation.is_empty() {
            Ok(())
        } else {
            Err(ScenarioError::SchemaValidationFailed {
                errors: validation.iter().map(|e| e.title.to_string()).collect(),
            })
        }
    }
}
