use eure::query::{TextFile, get_definition};
use eure::value::Text;
use query_flow::Db;

use crate::parser::DefinitionItem;
use crate::scenarios::{Scenario, ScenarioError};

#[derive(Debug, Clone)]
pub struct DefinitionScenario {
    pub editor: TextFile,
    /// Byte offset after stripping the editor's cursor marker.
    pub cursor: u32,
    pub definitions: Vec<DefinitionItem>,
}

impl Scenario for DefinitionScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let mut actual = Vec::new();
        for definition in get_definition(db, &self.editor, self.cursor)? {
            let source = db.asset(definition.file.clone())?;
            let mut target = source.get().to_string();
            target.insert_str(definition.selection.start as usize, "|_|");
            actual.push(DefinitionItem {
                file: definition.file.to_string(),
                target: Text::plaintext(target),
            });
        }
        let matches = actual.len() == self.definitions.len()
            && actual
                .iter()
                .zip(&self.definitions)
                .all(|(actual, expected)| {
                    actual.file == expected.file
                        && actual.target.as_str() == expected.target.as_str()
                });
        if !matches {
            return Err(ScenarioError::DefinitionsMismatch {
                expected: self.definitions,
                actual,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure::query::{TextFileContent, build_runtime};
    use query_flow::DurabilityLevel;

    #[test]
    fn rejects_wrong_file_marker_source_and_result_counts() {
        let runtime = build_runtime();
        let editor = TextFile::from_path("/ws/editor.eure".into());
        let source = "$schema = \"schema.eure\"\nname = \"Ada\"";
        runtime.resolve_asset(
            editor.clone(),
            TextFileContent(source.into()),
            DurabilityLevel::Static,
        );
        runtime.resolve_asset(
            TextFile::from_path("/ws/schema.eure".into()),
            TextFileContent("name = `text`\nage = `integer`\n".into()),
            DurabilityLevel::Static,
        );
        let expected = DefinitionItem {
            file: "/ws/schema.eure".into(),
            target: Text::plaintext("|_|name = `text`\nage = `integer`\n"),
        };
        let scenario = DefinitionScenario {
            editor,
            cursor: source.find("name").unwrap() as u32,
            definitions: vec![expected.clone()],
        };
        scenario.clone().run(&runtime).unwrap();
        let mut wrong_file = expected.clone();
        wrong_file.file = "/ws/other.eure".into();
        for definitions in [
            vec![wrong_file],
            vec![DefinitionItem {
                target: Text::plaintext("name = `text`\n|_|age = `integer`\n"),
                ..expected.clone()
            }],
            vec![DefinitionItem {
                target: Text::plaintext("|_|name = `integer`\nage = `integer`\n"),
                ..expected.clone()
            }],
            vec![DefinitionItem {
                target: Text::plaintext("name = `text`\nage = `integer`\n"),
                ..expected.clone()
            }],
            vec![],
            vec![expected.clone(), expected],
        ] {
            let incorrect = DefinitionScenario {
                definitions,
                ..scenario.clone()
            };
            assert!(matches!(
                incorrect.run(&runtime),
                Err(ScenarioError::DefinitionsMismatch { .. })
            ));
        }
    }
}
