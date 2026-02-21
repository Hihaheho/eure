use eure::query::{GetValidationErrorsFormattedExplicit, TextFile};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError, compare_error_lists};

pub struct SchemaErrorValidationScenario {
    pub input: TextFile,
    pub schema: TextFile,
    pub expected_errors: Vec<String>,
}

impl Scenario for SchemaErrorValidationScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let errors = db.query(GetValidationErrorsFormattedExplicit::new(
            self.input.clone(),
            self.schema.clone(),
        ))?;
        compare_error_lists(&errors, self.expected_errors)
    }
}
