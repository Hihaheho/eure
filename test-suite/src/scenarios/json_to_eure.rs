use eure::query::{ParseDocument, TextFile};
use eure_json::{Config, JsonToEure};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct JsonToEureScenario {
    pub input_json: TextFile,
    pub expected: TextFile,
    pub source_name: &'static str,
}

impl Scenario for JsonToEureScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual_doc = db.query(JsonToEure::new(self.input_json.clone(), Config::default()))?;

        // Parse the expected Eure document
        let expected_parsed = db.query(ParseDocument::new(self.expected.clone()))?;

        if *actual_doc != *expected_parsed.doc {
            return Err(ScenarioError::JsonToEureMismatch {
                expected_debug: format!("{:#?}", expected_parsed.doc),
                actual_debug: format!("{:#?}", actual_doc),
            });
        }

        Ok(())
    }
}
