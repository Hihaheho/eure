use std::collections::HashMap;

/// Represents a single test case with multiple test scenarios
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub description: Option<String>,
    pub scenarios: HashMap<String, TestScenario>,
}

/// A single test scenario within a test case
#[derive(Debug, Clone)]
pub struct TestScenario {
    pub kind: ScenarioKind,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScenarioKind {
    /// Input EURE source code
    Input,
    /// Normalized form - document as a single object
    Normalized,
    /// Expected JSON output
    Json,
    /// Expected error (for negative test cases)
    Error,
    /// Other custom scenario types
    Custom(String),
}

impl ScenarioKind {
    pub fn from_name(name: &str) -> Self {
        match name {
            "input" => Self::Input,
            "normalized" => Self::Normalized,
            "json" => Self::Json,
            "error" => Self::Error,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Input => "input",
            Self::Normalized => "normalized",
            Self::Json => "json",
            Self::Error => "error",
            Self::Custom(s) => s,
        }
    }
}

impl TestCase {
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            scenarios: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn add_scenario(&mut self, kind: ScenarioKind, content: String) {
        self.scenarios.insert(
            kind.as_str().to_string(),
            TestScenario { kind, content },
        );
    }

    pub fn get_scenario(&self, kind: &ScenarioKind) -> Option<&TestScenario> {
        self.scenarios.get(kind.as_str())
    }

    pub fn has_input(&self) -> bool {
        self.scenarios.contains_key("input")
    }

    pub fn has_normalized(&self) -> bool {
        self.scenarios.contains_key("normalized")
    }

    pub fn has_json(&self) -> bool {
        self.scenarios.contains_key("json")
    }
}
