/// Represents a single test case with multiple test scenarios
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub description: Option<String>,
    pub input: Option<String>,
    pub normalized: Option<String>,
    pub json: Option<String>,
    pub error: Option<String>,
}

impl TestCase {
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            input: None,
            normalized: None,
            json: None,
            error: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_input(mut self, input: String) -> Self {
        self.input = Some(input);
        self
    }

    pub fn with_normalized(mut self, normalized: String) -> Self {
        self.normalized = Some(normalized);
        self
    }

    pub fn with_json(mut self, json: String) -> Self {
        self.json = Some(json);
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }

    pub fn input_scenario(&self) -> Option<&str> {
        self.input.as_deref()
    }

    pub fn normalized_scenario(&self) -> Option<&str> {
        self.normalized.as_deref()
    }

    pub fn json_scenario(&self) -> Option<&str> {
        self.json.as_deref()
    }

    pub fn error_scenario(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
