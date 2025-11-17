use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod parser;
pub mod runner;
pub mod test_case;

pub use parser::parse_test_case;
pub use runner::TestRunner;
pub use test_case::TestCase;

/// The result of running all test cases
#[derive(Debug)]
pub struct TestResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub failures: Vec<TestFailure>,
}

impl TestResults {
    pub fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            failures: Vec::new(),
        }
    }

    pub fn add_pass(&mut self) {
        self.total += 1;
        self.passed += 1;
    }

    pub fn add_failure(&mut self, failure: TestFailure) {
        self.total += 1;
        self.failed += 1;
        self.failures.push(failure);
    }

    pub fn is_success(&self) -> bool {
        self.failed == 0
    }
}

impl Default for TestResults {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub test_name: String,
    pub error: String,
}

impl TestFailure {
    pub fn new(test_name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            test_name: test_name.into(),
            error: error.into(),
        }
    }
}
