use eure::query::{ParseCst, TextFile, read_text_file};
use eure_fmt::{FormatConfig, format_cst};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct FormattingScenario {
    pub input: TextFile,
    pub expected: TextFile,
}

impl Scenario for FormattingScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        // Get input source and CST
        let input_source = read_text_file(db, self.input.clone())?;
        let input_cst = db.query(ParseCst::new(self.input.clone()))?;

        // Get expected source
        let expected_source = read_text_file(db, self.expected.clone())?;

        // Format the input
        let config = FormatConfig::default();
        let actual = format_cst(&input_source, &input_cst.cst, &config);

        if actual != expected_source {
            return Err(ScenarioError::FormattingMismatch {
                input: input_source,
                expected: expected_source,
                actual,
            });
        }

        Ok(())
    }
}
