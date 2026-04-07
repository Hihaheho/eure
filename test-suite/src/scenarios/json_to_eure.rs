use std::sync::Arc;

use eure::query::{DocumentToSchemaQuery, ParseDocument, TextFile, TextFileContent};
use eure_json::{Config, JsonToEure};
use query_flow::Db;
use serde_eure::from_deserializer;

use crate::scenarios::{Scenario, ScenarioError};

pub struct JsonToEureScenario {
    pub input_json: TextFile,
    pub expected: TextFile,
    pub schema: Option<TextFile>,
    pub source_name: &'static str,
}

impl Scenario for JsonToEureScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let expected_parsed = db.query(ParseDocument::new(self.expected.clone()))?;

        if let Some(schema_file) = self.schema {
            // Schema-guided conversion via serde_eure.
            // Use a streaming serde_json::Deserializer rather than serde_json::Value so that
            // we exercise the format-agnostic typed-dispatch path (ADR-0016 §2).
            let schema_doc = db.query(DocumentToSchemaQuery::new(schema_file))?;
            let json_content: Arc<TextFileContent> = db
                .asset(self.input_json.clone())
                .map_err(|_| ScenarioError::FileNotFound(self.input_json.clone()))?;
            let mut de = serde_json::Deserializer::from_str(json_content.get());
            let actual_doc = from_deserializer(&mut de, &schema_doc.schema).map_err(|e| {
                ScenarioError::JsonToEureConversionError {
                    message: e.to_string(),
                }
            })?;
            if actual_doc != *expected_parsed.doc {
                return Err(ScenarioError::JsonToEureMismatch {
                    expected_debug: format!("{:#?}", expected_parsed.doc),
                    actual_debug: format!("{:#?}", actual_doc),
                });
            }
        } else {
            // Plain conversion
            let actual_doc =
                db.query(JsonToEure::new(self.input_json.clone(), Config::default()))?;
            if *actual_doc != *expected_parsed.doc {
                return Err(ScenarioError::JsonToEureMismatch {
                    expected_debug: format!("{:#?}", expected_parsed.doc),
                    actual_debug: format!("{:#?}", actual_doc),
                });
            }
        }

        Ok(())
    }
}
