use eure::query::{ParseDocument, TextFile};
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct NormalizationScenario {
    pub input: TextFile,
    pub normalized: TextFile,
}

impl Scenario for NormalizationScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let input_doc = db.query(ParseDocument::new(self.input.clone()))?;
        let normalized_doc = db.query(ParseDocument::new(self.normalized.clone()))?;
        if input_doc.doc != normalized_doc.doc {
            return Err(ScenarioError::NormalizationMismatch {
                input_debug: format!("{:#?}", input_doc.doc),
                normalized_debug: format!("{:#?}", normalized_doc.doc),
            });
        }
        Ok(())
    }
}
