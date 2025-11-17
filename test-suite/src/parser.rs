use anyhow::{Context, Result, anyhow};
use std::path::Path;

use crate::test_case::{TestCase, ScenarioKind};

/// Parse a test case file (EURE format with embedded code blocks)
pub fn parse_test_case(path: &Path, content: &str) -> Result<TestCase> {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut test_case = TestCase::new(name);

    // Parse EURE document to extract code blocks
    let cst = eure_parol::parse(content)
        .context("Failed to parse test case file")?;

    // Use ValueVisitor to convert CST to EureDocument
    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(content);
    cst.visit_from_root(&mut visitor)
        .map_err(|e| anyhow!("Failed to visit CST: {:?}", e))?;

    let document = visitor.into_document();
    let value = eure_tree::value_visitor::document_to_value(document);

    // Extract scenarios from the value
    if let eure_value::value::Value::Map(map) = value {
        for (key, val) in map.0.iter() {
            if let eure_value::value::ObjectKey::String(key_str) = key {
                // Check if this is a code block
                if let eure_value::value::Value::Code(code) = val {
                    let kind = ScenarioKind::from_name(key_str);
                    test_case.add_scenario(kind, code.content.clone());
                }
                // Also check for description
                else if key_str == "description" {
                    if let eure_value::value::Value::String(desc) = val {
                        test_case = test_case.with_description(desc.clone());
                    }
                }
            }
        }
    }

    Ok(test_case)
}
