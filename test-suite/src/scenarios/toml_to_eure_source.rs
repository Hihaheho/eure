use eure::query::{TextFile, read_text_file};
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
        let expected = read_text_file(db, self.input_eure.clone())?;

        if *actual != expected {
            return Err(ScenarioError::TomlToEureSourceMismatch {
                expected,
                actual: actual.as_ref().clone(),
            });
        }

        Ok(())
    }
}
