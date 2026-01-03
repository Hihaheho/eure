use eure::query::TextFile;
use eure_json_schema::{ConversionError, EureSchemaToJsonSchemaQuery};
use query_flow::{Db, QueryResultExt};

use crate::scenarios::{Scenario, ScenarioError};

pub struct EureSchemaToJsonSchemaErrorScenario {
    pub schema: TextFile,
    pub expected_errors: Vec<String>,
}

impl Scenario for EureSchemaToJsonSchemaErrorScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        match db
            .query(EureSchemaToJsonSchemaQuery::new(self.schema.clone()))
            .downcast_err::<ConversionError>()
        {
            Ok(Ok(_)) => Err(ScenarioError::ExpectedJsonSchemaConversionToFail {
                expected_errors: self.expected_errors,
            }),
            Ok(Err(e)) => {
                let error_message = e.to_string();
                // Compare exactly (only supports single error for now)
                let expected = self.expected_errors.join("\n").to_string();
                let actual = error_message.to_string();
                if actual == expected {
                    Ok(())
                } else {
                    Err(ScenarioError::SchemaConversionMismatch { expected, actual })
                }
            }
            Err(e) => Err(e.into()),
        }
    }
}
