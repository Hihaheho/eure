use eure::query::{GetValidationErrorsFormatted, TextFile};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError, compare_error_lists};

pub struct MetaSchemaScenario {
    pub schema: TextFile,
    pub meta_schema: TextFile,
    pub expected_errors: Vec<String>,
}

impl Scenario for MetaSchemaScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let errors = db.query(GetValidationErrorsFormatted::new(
            self.schema.clone(),
            self.meta_schema.clone(),
        ))?;

        if self.expected_errors.is_empty() {
            // Expect validation to pass
            if errors.is_empty() {
                Ok(())
            } else {
                Err(ScenarioError::SchemaValidationFailed {
                    errors: errors.iter().cloned().collect(),
                })
            }
        } else {
            // Expect validation to fail with specific errors
            compare_error_lists(&errors, self.expected_errors)
        }
    }
}
