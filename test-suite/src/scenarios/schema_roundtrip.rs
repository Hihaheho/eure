use eure::document::cst_to_document;
use eure::parol::parse;
use eure::query::{FormatSchema, TextFile};
use query_flow::Db;

use crate::parser::SchemaRoundtripMode;
use crate::scenarios::{Scenario, ScenarioError};

pub struct SchemaRoundtripScenario {
    pub schema: TextFile,
    pub mode: SchemaRoundtripMode,
}

impl Scenario for SchemaRoundtripScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Get formatted schema
        let formatted = db.query(FormatSchema::new(self.schema.clone()))?;

        // Get original schema source
        let original = db.asset(self.schema.clone())?.suspend()?;

        match self.mode {
            SchemaRoundtripMode::Format => {
                // String comparison (formatted output must match exactly)
                if *formatted != original.get() {
                    return Err(ScenarioError::SchemaRoundtripMismatch {
                        expected: original.get().to_string(),
                        actual: formatted.as_ref().clone(),
                    });
                }
            }
            SchemaRoundtripMode::Document => {
                // Document comparison (parsed documents must be equal)
                let original_doc = parse_to_document(original.get())?;
                let formatted_doc = parse_to_document(&formatted)?;

                if original_doc != formatted_doc {
                    return Err(ScenarioError::SchemaRoundtripDocumentMismatch {
                        original_source: original.get().to_string(),
                        formatted_source: formatted.as_ref().clone(),
                        original_doc: format!("{:?}", original_doc),
                        formatted_doc: format!("{:?}", formatted_doc),
                    });
                }
            }
            SchemaRoundtripMode::Disabled => {
                // Should not reach here, but handle gracefully
            }
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
