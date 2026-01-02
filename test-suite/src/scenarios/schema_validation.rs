use eure::query::{TextFile, UnionTagMode};
use query_flow::Db;

use crate::parser::InputUnionTagMode;
use crate::scenarios::{Scenario, ScenarioError};

pub struct SchemaValidationScenario {
    pub input: TextFile,
    pub schema: TextFile,
    pub union_tag_mode: InputUnionTagMode,
}

impl Scenario for SchemaValidationScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        use eure::query::ValidateAgainstSchemaWithMode;

        let mode: UnionTagMode = self.union_tag_mode.into();
        let validation = db.query(ValidateAgainstSchemaWithMode::new(
            self.input.clone(),
            self.schema.clone(),
            mode,
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
