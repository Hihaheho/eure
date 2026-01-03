//! Query-flow queries for eure-json-schema.

use eure::query::{DocumentToSchemaQuery, TextFile};
use query_flow::{Db, QueryError, query};

use crate::eure_to_json_schema;

/// Convert an Eure schema file to JSON Schema.
///
/// This query combines:
/// - DocumentToSchemaQuery (to parse and validate the schema)
/// - eure_to_json_schema (to convert to JSON Schema format)
///
/// Returns the JSON Schema as a serde_json::Value.
#[query]
pub fn eure_schema_to_json_schema_query(
    db: &impl Db,
    schema_file: TextFile,
) -> Result<serde_json::Value, QueryError> {
    let validated = db.query(DocumentToSchemaQuery::new(schema_file))?;
    let json_schema = eure_to_json_schema(&validated.schema)?;
    Ok(serde_json::to_value(&json_schema)?)
}
