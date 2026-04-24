use std::path::PathBuf;

use eure_document::identifier::Identifier;
use eure_document::path::PathSegment;
use eure_document::plan::{Form, LayoutPlan, PlanError};
use eure_document::value::ObjectKey;
use eure_fmt::source::format_source_document;
use eure_schema::SchemaDocument;
use eure_schema::write::{SchemaWriteError, schema_to_document, schema_to_source_document};
use thiserror::Error;

use crate::parser::CaseFile;

pub const CASE_SCHEMA_FILENAME: &str = "test-suite-case.schema.eure";

#[derive(Debug, Error)]
pub enum CaseSchemaError {
    #[error("failed to render test-suite case schema: {0}")]
    Write(#[from] SchemaWriteError),
    #[error("failed to plan case schema layout: {0}")]
    Plan(#[from] PlanError),
}

pub fn case_schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/schemas")
        .join(CASE_SCHEMA_FILENAME)
}

pub fn generate_case_schema_source() -> Result<String, CaseSchemaError> {
    let schema = SchemaDocument::of::<CaseFile>();
    let plan = case_schema_plan(&schema)?;
    let source = schema_to_source_document(&schema, plan)?;
    Ok(ensure_trailing_newline(format_source_document(&source)))
}

fn case_schema_plan(schema: &SchemaDocument) -> Result<LayoutPlan, CaseSchemaError> {
    let doc = schema_to_document(schema)?;
    let mut builder = LayoutPlan::builder(doc);

    let types_ident: Identifier = "types".parse().expect("valid identifier");
    let types_path = [PathSegment::Extension(types_ident.clone())];
    builder.set_form_at(&types_path, Form::BindingBlock)?;

    for type_name in schema.types.keys() {
        let path = [
            PathSegment::Extension(types_ident.clone()),
            PathSegment::Value(ObjectKey::String(type_name.to_string())),
        ];
        builder.set_form_at(&path, Form::BindingBlock)?;
    }

    Ok(builder.build()?)
}

fn ensure_trailing_newline(mut source: String) -> String {
    if !source.ends_with('\n') {
        source.push('\n');
    }
    source
}

#[cfg(test)]
mod tests {
    use crate::CaseFile;
    use eure::document::parse_to_document;
    use eure_document::identifier::Identifier;
    use eure_schema::{
        SchemaDocument, SchemaNodeContent, convert::document_to_schema, validate::validate,
    };

    use super::{case_schema_path, generate_case_schema_source};

    #[test]
    fn committed_case_schema_is_synced() {
        let generated = generate_case_schema_source().expect("generate schema");
        let schema_path = case_schema_path();
        let committed = std::fs::read_to_string(&schema_path).unwrap_or_else(|err| {
            panic!(
                "failed to read committed schema at {}: {err}",
                schema_path.display()
            )
        });

        assert_eq!(
            committed, generated,
            "committed schema is stale; run `cargo run -p test-suite --bin generate-case-schema`"
        );
    }

    #[test]
    fn case_schema_model_preserves_optional_struct_variant_fields() {
        let schema = SchemaDocument::of::<CaseFile>();

        assert_variant_record_field_optional(&schema, "json-data", "Separate", "input_json");
        assert_variant_record_field_optional(&schema, "json-data", "Separate", "output_json");
        assert_variant_record_field_optional(
            &schema,
            "json-schema-data",
            "Separate",
            "input_json_schema",
        );
        assert_variant_record_field_optional(
            &schema,
            "json-schema-data",
            "Separate",
            "output_json_schema",
        );
    }

    #[test]
    fn generated_case_schema_source_preserves_optional_struct_variant_fields() {
        let schema = parse_generated_case_schema();

        assert_variant_record_field_optional(&schema, "json-data", "Separate", "input_json");
        assert_variant_record_field_optional(&schema, "json-data", "Separate", "output_json");
        assert_variant_record_field_optional(
            &schema,
            "json-schema-data",
            "Separate",
            "input_json_schema",
        );
        assert_variant_record_field_optional(
            &schema,
            "json-schema-data",
            "Separate",
            "output_json_schema",
        );
    }

    #[test]
    fn generated_case_schema_enforces_exclusive_json_fixture_shapes() {
        let schema = parse_generated_case_schema();

        let shared =
            parse_to_document("json = json`{\"ok\":true}`", "<input>").expect("parse shared case");
        assert!(validate(&shared, &schema).is_valid);

        let split = parse_to_document("input_json = json`{\"ok\":true}`", "<input>")
            .expect("parse split case");
        assert!(validate(&split, &schema).is_valid);

        let mixed = parse_to_document(
            "json = json`{\"ok\":true}`\ninput_json = json`{\"ok\":true}`",
            "<input>",
        )
        .expect("parse mixed case");
        assert!(!validate(&mixed, &schema).is_valid);
    }

    fn assert_variant_record_field_optional(
        schema: &eure_schema::SchemaDocument,
        type_name: &str,
        variant_name: &str,
        field_name: &str,
    ) {
        let type_name = type_name.parse::<Identifier>().expect("valid type name");
        let schema_id = *schema
            .types
            .get(&type_name)
            .unwrap_or_else(|| panic!("missing type `{type_name}`"));
        let union = match &schema.node(schema_id).content {
            SchemaNodeContent::Union(union) => union,
            other => panic!("expected union for `{type_name}`, got {other:?}"),
        };
        let variant_schema_id = *union
            .variants
            .get(variant_name)
            .unwrap_or_else(|| panic!("missing variant `{variant_name}` in `{type_name}`"));
        let record = match &schema.node(variant_schema_id).content {
            SchemaNodeContent::Record(record) => record,
            other => panic!("expected record for `{type_name}.{variant_name}`, got {other:?}"),
        };
        let field = record.properties.get(field_name).unwrap_or_else(|| {
            panic!("missing field `{field_name}` in `{type_name}.{variant_name}`")
        });
        assert!(
            field.optional,
            "expected `{type_name}.{variant_name}.{field_name}` to be optional"
        );
    }

    fn parse_generated_case_schema() -> SchemaDocument {
        let generated = generate_case_schema_source().expect("generate schema");
        let doc = parse_to_document(&generated, "<input>").expect("parse generated schema source");
        let (schema, _) = document_to_schema(&doc).expect("convert generated schema");
        schema
    }
}
