use std::sync::Arc;

use eure::query::{TextFile, TextFileContent};
use eure_json::{Config, EureToJson, JsonToEureError};
use query_flow::{Db, QueryResultExt};

use crate::scenarios::{Scenario, ScenarioError};

pub struct EureToJsonScenario {
    pub input: TextFile,
    pub output_json: TextFile,
    pub source_name: &'static str,
}

impl Scenario for EureToJsonScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Parse the input Eure document
        let actual = db
            .query(EureToJson::new(self.input.clone(), Config::default()))
            .downcast_err::<JsonToEureError>()?
            .map_err(|e| ScenarioError::EureToJsonConversionError {
                message: e.to_string(),
            })?;

        // Read the expected JSON file content directly
        let expected_json: Arc<TextFileContent> = db
            .asset(self.output_json.clone())
            .map_err(|_| ScenarioError::FileNotFound(self.output_json.clone()))?
            .suspend()
            .map_err(|_| ScenarioError::FileNotFound(self.output_json.clone()))?;

        // Parse expected JSON
        let expected: serde_json::Value =
            serde_json::from_str(expected_json.get()).map_err(|e| {
                ScenarioError::JsonParseError {
                    message: e.to_string(),
                }
            })?;

        if *actual != expected {
            return Err(ScenarioError::EureToJsonMismatch {
                expected: Box::new(expected),
                actual,
            });
        }

        Ok(())
    }
}
