//! EURE Schema validation library
//!
//! This library provides schema extraction and validation for EURE documents.
//! It supports both standalone schema files and inline schemas within documents.

mod builder;
mod document_schema;
mod document_validator;
pub mod error;
mod identifiers;
mod impls;
mod schema;
mod utils;
mod value_api;

pub use builder::{FieldSchemaBuilder, ObjectSchemaBuilder, TypeBuilder, VariantSchemaBuilder};
pub use document_schema::{SchemaError, document_to_schema, is_pure_schema_node};
pub use document_validator::{Severity, ValidationError, ValidationErrorKind, validate_document};
pub use error::ValueError;
pub use eure_value::value::{KeyCmpValue, PathSegment};
pub use schema::*;
pub use utils::{
    path_segments_to_display_string, path_to_display_string, to_camel_case, to_kebab_case,
    to_pascal_case, to_snake_case,
};
pub use value_api::{
    ExtractedSchema, ValidationResult, extract_schema_from_value, validate_and_extract_schema,
    validate_self_describing, validate_with_schema_value, validate_with_tree,
};

// Re-export the derive macro if the feature is enabled
#[cfg(feature = "derive")]
pub use eure_derive::Eure;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_schema() {
        // TODO: Add tests once we have a parser
        // For now, we can test schema construction
        let schema = DocumentSchema::default();
        assert!(schema.types.is_empty());
        assert!(schema.root.fields.is_empty());
        assert!(schema.cascade_types.is_empty());
    }

    #[test]
    fn test_rename_rules() {
        assert_eq!(RenameRule::CamelCase.apply("snake_case"), "snakeCase");
        assert_eq!(RenameRule::SnakeCase.apply("camelCase"), "camel_case");
        assert_eq!(RenameRule::KebabCase.apply("camelCase"), "camel-case");
        assert_eq!(RenameRule::PascalCase.apply("snake_case"), "SnakeCase");
        assert_eq!(RenameRule::Lowercase.apply("UPPERCASE"), "uppercase");
        assert_eq!(RenameRule::Uppercase.apply("lowercase"), "LOWERCASE");
    }

    #[test]
    fn test_validation_error_severity() {
        use crate::document_validator::{Severity, ValidationError, ValidationErrorKind};
        use eure_tree::document::NodeId;

        let type_error = ValidationError {
            kind: ValidationErrorKind::TypeMismatch {
                expected: "string".to_string(),
                actual: "number".to_string(),
            },
            severity: Severity::Error,
            node_id: NodeId(0),
        };
        assert_eq!(type_error.severity, Severity::Error);
    }
}
