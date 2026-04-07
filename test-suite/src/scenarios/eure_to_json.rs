use std::sync::Arc;

use eure::query::{DocumentToSchemaQuery, ParseDocument, TextFile, TextFileContent};
use eure_json::{Config, EureToJson, JsonToEureError};
use query_flow::{Db, QueryResultExt};
use serde_eure::to_serializer;

use crate::scenarios::{Scenario, ScenarioError};

pub struct EureToJsonScenario {
    pub input: TextFile,
    pub output_json: TextFile,
    pub schema: Option<TextFile>,
    pub source_name: &'static str,
}

impl Scenario for EureToJsonScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual: serde_json::Value = if let Some(schema_file) = self.schema {
            // Schema-guided conversion via serde_eure
            let parsed = db.query(ParseDocument::new(self.input.clone()))?;
            let schema_doc = db.query(DocumentToSchemaQuery::new(schema_file))?;
            to_serializer(
                serde_json::value::Serializer,
                &parsed.doc,
                &schema_doc.schema,
            )
            .map_err(|e| ScenarioError::EureToJsonConversionError {
                message: e.to_string(),
            })?
        } else {
            // Plain conversion
            let arc_val = db
                .query(EureToJson::new(self.input.clone(), Config::default()))
                .downcast_err::<JsonToEureError>()?
                .map_err(|e| ScenarioError::EureToJsonConversionError {
                    message: e.to_string(),
                })?;
            Arc::unwrap_or_clone(arc_val)
        };

        // Read the expected JSON file content directly
        let expected_json: Arc<TextFileContent> = db
            .asset(self.output_json.clone())
            .map_err(|_| ScenarioError::FileNotFound(self.output_json.clone()))?;

        // Parse expected JSON
        let expected: serde_json::Value =
            serde_json::from_str(expected_json.get()).map_err(|e| {
                ScenarioError::JsonParseError {
                    message: e.to_string(),
                }
            })?;

        if actual != expected {
            return Err(ScenarioError::EureToJsonMismatch {
                expected: Box::new(expected),
                actual: Arc::new(actual),
            });
        }

        Ok(())
    }
}
