use eure::query::{DocumentToSchemaQuery, TextFile};
use eure_codegen::{GenerationConfig, emit_rust_types, schema_to_ir_module};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct RustCodegenScenario {
    pub schema: TextFile,
    pub expected_rust: TextFile,
}

impl Scenario for RustCodegenScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let validated = db
            .query(DocumentToSchemaQuery::new(self.schema.clone()))
            .map_err(|e| ScenarioError::RustCodegenError {
                message: e.to_string(),
            })?;

        let module = schema_to_ir_module(&validated.schema).map_err(|e| {
            ScenarioError::RustCodegenError {
                message: e.to_string(),
            }
        })?;

        let actual = emit_rust_types(
            &module,
            &GenerationConfig::builder().allow_warnings(false).build(),
        )
        .map_err(|e| ScenarioError::RustCodegenError {
            message: e.to_string(),
        })?;

        let expected = db.asset(self.expected_rust.clone())?;
        let expected = expected.get().trim().to_string();
        let actual = actual.trim().to_string();

        if actual == expected {
            Ok(())
        } else {
            Err(ScenarioError::RustCodegenMismatch { expected, actual })
        }
    }
}
