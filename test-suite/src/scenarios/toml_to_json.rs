use std::sync::Arc;

use eure::query::TextFile;
use eure_json::{Config, EureToJson, document_to_value};
use eure_toml::TomlToEureDocument;
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct TomlToJsonScenario {
    pub input_toml: TextFile,
    pub input_eure: TextFile,
}

impl Scenario for TomlToJsonScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let toml_doc = db.query(TomlToEureDocument::new(self.input_toml.clone()))?;

        // Convert to JSON from TOML-derived document
        let config = Config::default();
        let toml_json = document_to_value(&toml_doc, &config).map_err(|e| {
            ScenarioError::EureToJsonConversionError {
                message: e.to_string(),
            }
        })?;

        // Convert the Eure file to JSON via query
        let eure_json = db.query(EureToJson::new(self.input_eure.clone(), config))?;

        // Compare JSON outputs
        if toml_json != *eure_json {
            return Err(ScenarioError::EureToJsonMismatch {
                expected: Box::new(eure_json.as_ref().clone()),
                actual: Arc::new(toml_json),
            });
        }

        Ok(())
    }
}
