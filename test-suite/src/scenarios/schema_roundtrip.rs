use eure::document::cst_to_document;
use eure::parol::parse;
use eure::query::{FormatSchema, TextFile};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

/// Schema roundtrip scenario - always runs when schema is present.
/// Tests that schema semantics are preserved after format_schema roundtrip.
pub struct SchemaRoundtripScenario {
    pub schema: TextFile,
}

impl Scenario for SchemaRoundtripScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Get formatted schema
        let formatted = db.query(FormatSchema::new(self.schema.clone()))?;

        // Get original schema source
        let original = db.asset(self.schema.clone())?.suspend()?;

        // Parse both to EureDocument and compare
        let original_doc = parse_to_document(original.get())?;
        let formatted_doc = parse_to_document(&formatted)?;

        if original_doc != formatted_doc {
            return Err(ScenarioError::SchemaRoundtripMismatch {
                original_source: original.get().to_string(),
                formatted_source: formatted.as_ref().clone(),
                original_doc: format!("{:#?}", original_doc),
                formatted_doc: format!("{:#?}", formatted_doc),
            });
        }
        Ok(())
    }
}

fn parse_to_document(input: &str) -> Result<eure::document::EureDocument, ScenarioError> {
    let cst = parse(input).map_err(|e| ScenarioError::PreprocessingError {
        message: format!("Parse error: {}", e),
    })?;
    cst_to_document(input, &cst).map_err(|e| ScenarioError::PreprocessingError {
        message: format!("Document construction error: {}", e),
    })
}
