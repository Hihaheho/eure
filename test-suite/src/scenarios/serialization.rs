use eure::query::{DocumentToSchemaQuery, ParseDocument, TextFile};
use eure_fmt::format_source_document;
use eure_schema::type_path_trace::materialize_layout_plan;
use eure_schema::validate::validate_with_trace;
use query_flow::Db;

use crate::scenarios::{Scenario, ScenarioError};

pub struct SerializationScenario {
    pub input: TextFile,
    pub schema: TextFile,
    pub expected: TextFile,
}

impl Scenario for SerializationScenario {
    fn run(self, db: &impl Db) -> Result<(), ScenarioError> {
        let input_source = db.asset(self.input.clone())?;
        let expected_source = db.asset(self.expected.clone())?;

        let input_doc = db.query(ParseDocument::new(self.input.clone()))?;
        let schema_doc = db.query(DocumentToSchemaQuery::new(self.schema.clone()))?;

        let trace = validate_with_trace(
            &input_doc.doc,
            &schema_doc.schema,
            &schema_doc.layout.schema_node_paths,
        );
        let plan = materialize_layout_plan(
            (*input_doc.doc).clone(),
            &trace.node_type_traces,
            &schema_doc.layout,
        )
        .map_err(ScenarioError::LayoutPlan)?;
        let source = plan.emit();
        let actual = format_source_document(&source);

        if actual != expected_source.get() {
            return Err(ScenarioError::SerializationMismatch {
                input: input_source.get().to_string(),
                expected: expected_source.get().to_string(),
                actual,
            });
        }

        Ok(())
    }
}
