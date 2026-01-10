use eure::document::cst_to_document;
use eure::parol::parse;
use eure::query::{FormatSchema, TextFile};
use eure_schema::convert::document_to_schema;
use query_flow::Db;

use crate::parser::SchemaRoundtripMode;
use crate::scenarios::{Scenario, ScenarioError};

/// Schema roundtrip scenario - always runs when schema is present.
/// Tests that schema semantics are preserved after format_schema roundtrip.
pub struct SchemaRoundtripScenario {
    pub schema: TextFile,
    pub mode: SchemaRoundtripMode,
}

impl Scenario for SchemaRoundtripScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        match self.mode {
            SchemaRoundtripMode::Document => self.run_document_mode(db),
            SchemaRoundtripMode::Schema => self.run_schema_mode(db),
            SchemaRoundtripMode::Format => self.run_format_mode(db),
        }
    }
}

impl SchemaRoundtripScenario {
    /// Document mode: Compare parsed EureDocuments
    fn run_document_mode(self, db: &impl Db) -> Result<(), ScenarioError> {
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

    /// Schema mode: Compare schema(input) == schema(format(schema(input)))
    /// This allows document-level differences while verifying schema semantics are preserved.
    fn run_schema_mode(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Get formatted schema
        let formatted = db.query(FormatSchema::new(self.schema.clone()))?;

        // Get original schema source
        let original = db.asset(self.schema.clone())?.suspend()?;

        // Parse original to document and convert to schema
        let original_doc = parse_to_document(original.get())?;
        let (original_schema, _) =
            document_to_schema(&original_doc).map_err(|e| ScenarioError::PreprocessingError {
                message: format!("Schema conversion error (original): {}", e),
            })?;

        // Parse formatted to document and convert to schema
        let formatted_doc = parse_to_document(&formatted)?;
        let (formatted_schema, _) =
            document_to_schema(&formatted_doc).map_err(|e| ScenarioError::PreprocessingError {
                message: format!(
                    "Schema conversion error (formatted): {}\n\nFormatted output:\n{}",
                    e, formatted
                ),
            })?;

        // Compare schemas using PartialEq (ignores formatting hints and node ordering)
        if original_schema != formatted_schema {
            return Err(ScenarioError::SchemaRoundtripMismatch {
                original_source: original.get().to_string(),
                formatted_source: formatted.as_ref().clone(),
                original_doc: format!("{:#?}", original_schema),
                formatted_doc: format!("{:#?}", formatted_schema),
            });
        }
        Ok(())
    }

    /// Format mode: Compare input == format(schema(input))
    /// This verifies exact string equality (canonical format).
    fn run_format_mode(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Get formatted schema
        let formatted = db.query(FormatSchema::new(self.schema.clone()))?;

        // Get original schema source
        let original = db.asset(self.schema.clone())?.suspend()?;

        // Compare strings directly
        if original.get() != formatted.as_ref() {
            return Err(ScenarioError::SchemaRoundtripMismatch {
                original_source: original.get().to_string(),
                formatted_source: formatted.as_ref().clone(),
                original_doc: String::new(),
                formatted_doc: String::new(),
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
