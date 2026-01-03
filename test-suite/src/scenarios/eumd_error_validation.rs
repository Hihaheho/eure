use eure::query::TextFile;
use eure_mark::CheckEumdReferencesFormatted;
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError, compare_error_lists};

pub struct EumdErrorValidationScenario {
    pub input: TextFile,
    pub expected_errors: Vec<String>,
}

impl Scenario for EumdErrorValidationScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual = db.query(CheckEumdReferencesFormatted::new(self.input.clone()))?;
        compare_error_lists(&actual, self.expected_errors)
    }
}
