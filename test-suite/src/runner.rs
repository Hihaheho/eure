use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{parse_test_case, TestCase, TestResults, TestFailure};
use crate::test_case::ScenarioKind;

pub struct TestRunner {
    cases_dir: PathBuf,
}

impl TestRunner {
    pub fn new(cases_dir: impl Into<PathBuf>) -> Self {
        Self {
            cases_dir: cases_dir.into(),
        }
    }

    /// Discover and run all test cases
    pub fn run_all(&self) -> Result<TestResults> {
        let mut results = TestResults::new();

        // Find all .eure files in cases directory
        let test_files = self.discover_test_files()?;

        for test_file in test_files {
            let content = fs::read_to_string(&test_file)
                .with_context(|| format!("Failed to read test file: {:?}", test_file))?;

            match parse_test_case(&test_file, &content) {
                Ok(test_case) => {
                    self.run_test_case(&test_case, &mut results)?;
                }
                Err(e) => {
                    results.add_failure(TestFailure::new(
                        test_file.display().to_string(),
                        format!("Failed to parse test case: {}", e),
                    ));
                }
            }
        }

        Ok(results)
    }

    /// Discover all .eure test files
    fn discover_test_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.walk_directory(&self.cases_dir, &mut files)?;
        files.sort();
        Ok(files)
    }

    fn walk_directory(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.walk_directory(&path, files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("eure") {
                files.push(path);
            }
        }

        Ok(())
    }

    /// Run a single test case
    fn run_test_case(&self, test_case: &TestCase, results: &mut TestResults) -> Result<()> {
        // Run all applicable tests for this test case

        // Test 1: If both input and normalized exist, parse both and compare
        if test_case.has_input() && test_case.has_normalized() {
            if let Err(e) = self.test_input_normalized_equivalence(test_case) {
                results.add_failure(TestFailure::new(
                    format!("{} (input <-> normalized)", test_case.name),
                    e.to_string(),
                ));
                return Ok(());
            }
        }

        // Test 2: If both input and json exist, parse input and compare JSON
        if test_case.has_input() && test_case.has_json() {
            if let Err(e) = self.test_input_json_conversion(test_case) {
                results.add_failure(TestFailure::new(
                    format!("{} (input -> JSON)", test_case.name),
                    e.to_string(),
                ));
                return Ok(());
            }
        }

        // Test 3: If both normalized and json exist, convert and compare
        if test_case.has_normalized() && test_case.has_json() {
            if let Err(e) = self.test_normalized_json_conversion(test_case) {
                results.add_failure(TestFailure::new(
                    format!("{} (normalized -> JSON)", test_case.name),
                    e.to_string(),
                ));
                return Ok(());
            }
        }

        results.add_pass();
        Ok(())
    }

    /// Test that input and normalized are equivalent
    fn test_input_normalized_equivalence(&self, test_case: &TestCase) -> Result<()> {
        let input = test_case.get_scenario(&ScenarioKind::Input).unwrap();
        let normalized = test_case.get_scenario(&ScenarioKind::Normalized).unwrap();

        let input_doc = self.parse_to_document(&input.content)?;
        let normalized_doc = self.parse_to_document(&normalized.content)?;

        if input_doc != normalized_doc {
            anyhow::bail!(
                "Input and normalized documents are not equal.\nInput: {:?}\nNormalized: {:?}",
                input_doc,
                normalized_doc
            );
        }

        Ok(())
    }

    /// Test that input converts to expected JSON
    fn test_input_json_conversion(&self, test_case: &TestCase) -> Result<()> {
        let input = test_case.get_scenario(&ScenarioKind::Input).unwrap();
        let expected_json = test_case.get_scenario(&ScenarioKind::Json).unwrap();

        let value = self.parse_to_value(&input.content)?;
        let actual_json = eure_json::value_to_json(&value)
            .context("Failed to convert value to JSON")?;

        let expected: serde_json::Value = serde_json::from_str(&expected_json.content)
            .context("Failed to parse expected JSON")?;

        if actual_json != expected {
            anyhow::bail!(
                "JSON mismatch.\nExpected: {}\nActual: {}",
                serde_json::to_string_pretty(&expected)?,
                serde_json::to_string_pretty(&actual_json)?
            );
        }

        Ok(())
    }

    /// Test that normalized converts to expected JSON
    fn test_normalized_json_conversion(&self, test_case: &TestCase) -> Result<()> {
        let normalized = test_case.get_scenario(&ScenarioKind::Normalized).unwrap();
        let expected_json = test_case.get_scenario(&ScenarioKind::Json).unwrap();

        let value = self.parse_to_value(&normalized.content)?;
        let actual_json = eure_json::value_to_json(&value)
            .context("Failed to convert value to JSON")?;

        let expected: serde_json::Value = serde_json::from_str(&expected_json.content)
            .context("Failed to parse expected JSON")?;

        if actual_json != expected {
            anyhow::bail!(
                "JSON mismatch.\nExpected: {}\nActual: {}",
                serde_json::to_string_pretty(&expected)?,
                serde_json::to_string_pretty(&actual_json)?
            );
        }

        Ok(())
    }

    /// Parse EURE source to EureDocument
    fn parse_to_document(&self, source: &str) -> Result<eure_value::document::EureDocument> {
        let cst = eure_parol::parse(source)
            .context("Failed to parse EURE source")?;

        let mut visitor = eure_tree::value_visitor::ValueVisitor::new(source);
        cst.visit_from_root(&mut visitor)
            .map_err(|e| anyhow::anyhow!("Failed to visit CST: {:?}", e))?;

        Ok(visitor.into_document())
    }

    /// Parse EURE source to Value
    fn parse_to_value(&self, source: &str) -> Result<eure_value::value::Value> {
        let document = self.parse_to_document(source)?;
        Ok(eure_tree::value_visitor::document_to_value(document))
    }
}
