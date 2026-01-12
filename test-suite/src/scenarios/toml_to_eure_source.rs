use eure::query::TextFile;
use eure_toml::TomlToEureSource;
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct TomlToEureSourceScenario {
    pub input_toml: TextFile,
    pub input_eure: TextFile,
}

impl Scenario for TomlToEureSourceScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let actual = db.query(TomlToEureSource::new(self.input_toml.clone()))?;

        // Read expected Eure source directly
        let expected = db.asset(self.input_eure.clone().clone())?;

        if *actual != expected.get() {
            return Err(ScenarioError::TomlToEureSourceMismatch {
                expected: expected.get().to_string(),
                actual: actual.as_ref().clone(),
            });
        }

        Ok(())
    }
}
