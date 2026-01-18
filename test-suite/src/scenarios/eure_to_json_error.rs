use std::sync::Arc;

use eure::query::TextFile;
use eure::report::ErrorReports;
use eure_json::{Config, EureToJsonFormatted};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError, compare_error_lists};

pub struct EureToJsonErrorScenario {
    pub input: TextFile,
    pub expected_errors: Vec<String>,
}

impl Scenario for EureToJsonErrorScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Try EureToJsonFormatted to get formatted errors with spans
        let result = db.query(EureToJsonFormatted::new(
            self.input.clone(),
            Config::default(),
        ));

        match result {
            Ok(_) => {
                // Conversion succeeded but we expected errors
                Err(ScenarioError::ExpectedValidationToFail {
                    expected_errors: self.expected_errors,
                })
            }
            Err(e) => {
                // Get the error reports from the query error
                if let Some(reports) = e.downcast_ref::<ErrorReports>() {
                    // Format the errors for comparison
                    let formatted: Vec<String> = reports
                        .iter()
                        .map(|r| eure::report::format_error_report(db, r, false))
                        .collect::<Result<Vec<_>, _>>()?;
                    compare_error_lists(&Arc::new(formatted), self.expected_errors)
                } else {
                    // Fallback: unexpected error type
                    Err(ScenarioError::EureToJsonConversionError {
                        message: e.to_string(),
                    })
                }
            }
        }
    }
}
