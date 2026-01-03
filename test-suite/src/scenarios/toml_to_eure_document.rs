use eure::query::{ParseDocument, TextFile};
use eure_toml::TomlToEureDocument;
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct TomlToEureDocumentScenario {
    pub input_toml: TextFile,
    pub input_eure: TextFile,
}

impl Scenario for TomlToEureDocumentScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual_doc = db.query(TomlToEureDocument::new(self.input_toml.clone()))?;

        // Parse the expected Eure document
        let expected_parsed = db.query(ParseDocument::new(self.input_eure.clone()))?;

        // Compare the documents (ignoring layout)
        if *actual_doc != *expected_parsed.doc {
            return Err(ScenarioError::TomlToEureDocumentMismatch {
                expected_debug: format!("{:#?}", expected_parsed.doc),
                actual_debug: format!("{:#?}", actual_doc),
            });
        }

        Ok(())
    }
}
