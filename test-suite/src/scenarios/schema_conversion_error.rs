use eure::query::{DocumentToSchemaQuery, TextFile, WithFormattedError};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct SchemaConversionErrorScenario {
    pub schema: TextFile,
    pub expected_error: Option<String>,
}

impl Scenario for SchemaConversionErrorScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let result = db.query(WithFormattedError::new(
            DocumentToSchemaQuery::new(self.schema.clone()),
            false,
        ))?;

        match (result.as_ref(), &self.expected_error) {
            // Got error when expecting error
            (Err(actual_error), Some(expected)) => {
                let actual_trimmed = actual_error.trim();
                let expected_trimmed = expected.trim();
                if actual_trimmed == expected_trimmed {
                    Ok(())
                } else {
                    Err(ScenarioError::SchemaConversionMismatch {
                        expected: expected_trimmed.to_string(),
                        actual: actual_trimmed.to_string(),
                    })
                }
            }
            // Got error when not expecting error
            (Err(actual_error), None) => Err(ScenarioError::SchemaConversionError {
                message: actual_error.clone(),
            }),
            // No error when expecting error
            (Ok(_), Some(expected)) => Err(ScenarioError::ExpectedSchemaConversionToFail {
                expected_errors: vec![expected.clone()],
            }),
            // No error when not expecting error (success case)
            (Ok(_), None) => Ok(()),
        }
    }
}
