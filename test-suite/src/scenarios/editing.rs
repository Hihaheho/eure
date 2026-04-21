use std::str::FromStr;

use eros::ErrorUnion;
use eure::document::{DocumentConstructionError, parse_to_document};
use eure::edit::{EditCommand, EditPath, EditValue, EditableDocument};
use eure::parol::EureParseError;
use eure::query::TextFile;
use query_flow::Db;

use crate::parser::EditCommandFixture;
use crate::scenarios::{Scenario, ScenarioError};

pub struct EditingScenario {
    pub input: TextFile,
    pub expected: Option<TextFile>,
    pub commands: Vec<EditCommandFixture>,
}

impl Scenario for EditingScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let input_source = db.asset(self.input.clone())?;
        let expected_source = self
            .expected
            .clone()
            .map(|expected| db.asset(expected))
            .transpose()?;

        let mut document = EditableDocument::parse(input_source.get()).map_err(|error| {
            ScenarioError::EditingError {
                message: error.to_string(),
            }
        })?;

        for command in self.commands {
            let command = fixture_to_command(command)?;
            document
                .apply(command)
                .map_err(|error| ScenarioError::EditingError {
                    message: error.to_string(),
                })?;
        }

        if let Some(expected_source) = expected_source {
            let actual = document.render();
            if actual != expected_source.get() {
                return Err(ScenarioError::EditingMismatch {
                    input: input_source.get().to_string(),
                    expected: expected_source.get().to_string(),
                    actual,
                });
            }
        }

        Ok(())
    }
}

fn fixture_to_command(fixture: EditCommandFixture) -> Result<EditCommand, ScenarioError> {
    let path =
        EditPath::from_str(&fixture.path).map_err(|error| ScenarioError::EditPathParseError {
            message: error.to_string(),
        })?;
    match fixture.variant.as_str() {
        "Set" => Ok(EditCommand::Set {
            path,
            value: EditValue::Document(parse_required_fixture_value("Set", fixture.value)?),
        }),
        "Insert" => Ok(EditCommand::Insert {
            path,
            value: EditValue::Document(parse_required_fixture_value("Insert", fixture.value)?),
        }),
        "Delete" => Ok(EditCommand::Delete { path }),
        other => Err(ScenarioError::EditingError {
            message: format!("unknown edit command variant `{other}`"),
        }),
    }
}

fn parse_required_fixture_value(
    variant: &str,
    value: Option<eure::value::Text>,
) -> Result<eure::document::EureDocument, ScenarioError> {
    let value = value.ok_or_else(|| ScenarioError::EditingError {
        message: format!("edit command `{variant}` requires `value`"),
    })?;
    parse_fixture_value(value.as_str())
}

fn parse_fixture_value(input: &str) -> Result<eure::document::EureDocument, ScenarioError> {
    match parse_to_document(input) {
        Ok(doc) => Ok(doc),
        Err(first_error) => {
            let wrapped = format!("= {input}");
            parse_to_document(&wrapped).map_err(|_| ScenarioError::EditingError {
                message: format_parse_union_error(first_error),
            })
        }
    }
}

fn format_parse_union_error(
    error: ErrorUnion<(EureParseError, DocumentConstructionError)>,
) -> String {
    match error.narrow::<EureParseError, _>() {
        Ok(parse_error) => parse_error.to_string(),
        Err(error) => error.take::<DocumentConstructionError>().to_string(),
    }
}
