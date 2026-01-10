use eure::document::cst_to_document;
use eure::parol::parse;
use eure::query::{FormatSchema, TextFile};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

/// Document roundtrip scenario - compares EureDocuments.
/// Tests that: parse(input) == parse(format(schema(input)))
pub struct DocumentRoundtripScenario {
    pub schema: TextFile,
}

impl Scenario for DocumentRoundtripScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Get formatted schema
        let formatted = db.query(FormatSchema::new(self.schema.clone()))?;

        // Get original schema source
        let original = db.asset(self.schema.clone())?.suspend()?;

        // Parse both to EureDocument and compare
        let original_doc = parse_to_document(original.get())?;
        let formatted_doc = parse_to_document(&formatted)?;

        if original_doc != formatted_doc {
            return Err(ScenarioError::DocumentRoundtripMismatch {
                original_source: original.get().to_string(),
                formatted_source: formatted.as_ref().clone(),
            });
        }
        Ok(())
    }
}

fn parse_to_document(input: &str) -> Result<eure::document::EureDocument, ScenarioError> {
    let cst = parse(input).map_err(|e| ScenarioError::PreprocessingError {
        message: format!("Parse error: {}\n\nFormatted output:\n{}", e, input),
    })?;
    cst_to_document(input, &cst).map_err(|e| ScenarioError::PreprocessingError {
        message: format!(
            "Document construction error: {}\n\nFormatted output:\n{}",
            e, input
        ),
    })
}
