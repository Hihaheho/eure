use eure::query::{GetValidationErrorsFormattedWithMode, TextFile, UnionTagMode};
use query_flow::Db;

use crate::parser::InputUnionTagMode;
use crate::scenarios::{Scenario, ScenarioError, compare_error_lists};

pub struct SchemaErrorValidationScenario {
    pub input: TextFile,
    pub schema: TextFile,
    pub expected_errors: Vec<String>,
    pub union_tag_mode: InputUnionTagMode,
}

impl Scenario for SchemaErrorValidationScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let mode: UnionTagMode = self.union_tag_mode.into();
        let errors = db.query(GetValidationErrorsFormattedWithMode::new(
            self.input.clone(),
            self.schema.clone(),
            mode,
        ))?;
        compare_error_lists(&errors, self.expected_errors)
    }
}
