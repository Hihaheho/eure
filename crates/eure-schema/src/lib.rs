//! EURE Schema validation library
//! 
//! This library provides schema extraction and validation for EURE documents.
//! It supports both standalone schema files and inline schemas within documents.

mod schema;
mod impls;
mod builder;
mod utils;
mod value_schema;
mod value_validator;
mod tree_validator;

pub use schema::*;
pub use value_validator::{ValidationError, ValidationErrorKind, Severity};
pub use builder::{FieldSchemaBuilder, TypeBuilder, ObjectSchemaBuilder, VariantSchemaBuilder};
pub use utils::{to_camel_case, to_snake_case, to_pascal_case, to_kebab_case, path_to_display_string, path_segments_to_display_string};
pub use value_schema::{value_to_schema, is_pure_schema, SchemaError};
pub use value_validator::validate_document;
pub use eure_value::value::{PathSegment, KeyCmpValue};

// Compatibility aliases for old API names
pub use extract_schema_from_value as extract_schema;
pub use validate_with_schema_value as validate_with_schema;

// Re-export the derive macro if the feature is enabled
#[cfg(feature = "derive")]
pub use eure_derive::Eure;


/// Result of validating a self-describing document
pub struct ValidationResult {
    /// The extracted schema information
    pub schema: ExtractedSchema,
    /// Validation errors found
    pub errors: Vec<ValidationError>,
}

/// Macro to reduce boilerplate for parsing and value extraction
macro_rules! parse_and_extract_value {
    ($input:expr, $callback:expr) => {{
        // Parse to CST
        let tree = eure_parol::parse($input)
            .map_err(|e| SchemaError::InvalidField(format!("Parse error: {:?}", e)))?;
        
        // Extract to Value
        let mut values = eure_tree::value_visitor::Values::default();
        let mut visitor = eure_tree::value_visitor::ValueVisitor::new($input, &mut values);
        
        use eure_tree::prelude::*;
        tree.visit_from_root(&mut visitor)
            .map_err(|e| SchemaError::InvalidField(format!("Value extraction error: {:?}", e)))?;
        
        // Get document value
        let doc_value = if let Ok(root_view) = tree.root_handle().get_view(&tree) {
            values.get_eure(&root_view.eure)
                .ok_or_else(|| SchemaError::InvalidField("No document value found".to_string()))?
        } else {
            return Err(SchemaError::InvalidField("Invalid document structure".to_string()).into());
        };
        
        $callback(doc_value)
    }};
}

/// Extract schema information from a EURE document using Value-based approach
/// 
/// This is the new recommended way to extract schemas.
pub fn extract_schema_from_value(input: &str) -> Result<ExtractedSchema, SchemaError> {
    parse_and_extract_value!(input, |doc_value| {
        // Convert to schema
        let schema = value_to_schema(doc_value)?;
        let is_pure = is_pure_schema(doc_value);
        
        Ok(ExtractedSchema { 
            document_schema: schema,
            is_pure_schema: is_pure,
        })
    })
}


/// Validate a document against a provided schema using Value-based approach
/// 
/// This is the new recommended way to validate documents.
pub fn validate_with_schema_value(
    input: &str,
    schema: DocumentSchema,
) -> Result<Vec<ValidationError>, SchemaError> {
    parse_and_extract_value!(input, |doc_value| {
        // Validate
        Ok(validate_document(doc_value, &schema))
    })
}

/// Extract schema and validate in one pass for self-describing documents
/// 
/// This is useful for documents that contain their own schema definitions.
/// Pure schema documents (containing only $types definitions) are not validated.
/// 
/// # Arguments
/// * `input` - The input string
/// 
/// # Returns
/// The extracted schema and any validation errors
pub fn validate_self_describing(input: &str) -> Result<ValidationResult, Box<dyn std::error::Error>> {
    parse_and_extract_value!(input, |doc_value| {
        // Extract schema
        let schema = value_to_schema(doc_value)?;
        let is_pure = is_pure_schema(doc_value);
        
        // Validate if not a pure schema
        let errors = if is_pure {
            Vec::new()
        } else {
            validate_document(doc_value, &schema)
        };
        
        Ok(ValidationResult {
            schema: ExtractedSchema { 
                document_schema: schema,
                is_pure_schema: is_pure,
            },
            errors,
        })
    })
}

/// Create a validator function from a schema document
/// 
/// This is useful when you want to validate multiple documents against
/// the same schema.
/// 
/// # Arguments
/// * `input` - A EURE document string containing schema definitions
/// 
/// # Returns
/// A function that validates documents against the schema, or an error
/// if the provided document is not a pure schema.
pub fn create_validator(input: &str) -> Result<DocumentSchema, Box<dyn std::error::Error>> {
    let extracted = extract_schema_from_value(input)?;
    
    if !extracted.is_pure_schema {
        return Err("Provided document is not a pure schema".into());
    }
    
    Ok(extracted.document_schema)
}

/// Check if validation errors contain any errors (not just warnings)
pub fn has_errors(errors: &[ValidationError]) -> bool {
    errors.iter().any(|e| e.severity == Severity::Error)
}

/// Validate a document against a schema using the CST for span information
/// 
/// This is the preferred method when you need accurate span information
/// for error reporting (e.g., in LSP).
/// 
/// # Arguments
/// * `input` - The input string
/// * `schema` - The schema to validate against
/// * `tree` - The concrete syntax tree
/// 
/// # Returns
/// A list of validation errors with span information
pub fn validate_with_tree(
    input: &str,
    schema: DocumentSchema,
    tree: &eure_tree::tree::ConcreteSyntaxTree<eure_tree::node_kind::TerminalKind, eure_tree::node_kind::NonTerminalKind>,
) -> Result<Vec<ValidationError>, Box<dyn std::error::Error>> {
    use eure_tree::value_visitor::{ValueVisitor, Values};
    
    use crate::tree_validator::SchemaValidator;
    
    // First pass: Extract values using ValueVisitor
    let mut values = Values::default();
    let mut value_visitor = ValueVisitor::new(input, &mut values);
    tree.visit_from_root(&mut value_visitor)?;
    
    // Second pass: Validate with span tracking
    let mut validator = SchemaValidator::new(input, &schema, &values);
    tree.visit_from_root(&mut validator)?;
    
    Ok(validator.into_errors())
}

/// Validate a self-describing document using the CST for span information
/// 
/// # Arguments
/// * `input` - The input string
/// * `tree` - The concrete syntax tree
/// 
/// # Returns
/// The extracted schema and validation errors with span information
pub fn validate_self_describing_with_tree(
    input: &str,
    tree: &eure_tree::tree::ConcreteSyntaxTree<eure_tree::node_kind::TerminalKind, eure_tree::node_kind::NonTerminalKind>,
) -> Result<ValidationResult, Box<dyn std::error::Error>> {
    // Extract schema using existing value-based method
    let extracted = extract_schema_from_value(input)?;
    
    // Validate using tree-based method for spans if not a pure schema
    let errors = if extracted.is_pure_schema {
        Vec::new()
    } else {
        validate_with_tree(input, extracted.document_schema.clone(), tree)?
    };
    
    Ok(ValidationResult {
        schema: extracted,
        errors,
    })
}

/// Filter errors by severity
pub fn filter_by_severity(errors: &[ValidationError], severity: Severity) -> Vec<&ValidationError> {
    errors.iter()
        .filter(|e| e.severity == severity)
        .collect()
}

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
        assert!(schema.cascade_type.is_none());
    }

    #[test]
    fn test_type_from_path() {
        assert_eq!(Type::from_path(".string"), Some(Type::String));
        assert_eq!(Type::from_path(".number"), Some(Type::Number));
        assert_eq!(Type::from_path(".boolean"), Some(Type::Boolean));
        assert_eq!(Type::from_path(".null"), Some(Type::Null));
        assert_eq!(Type::from_path(".any"), Some(Type::Any));
        assert_eq!(Type::from_path(".path"), Some(Type::Path));
        assert_eq!(Type::from_path(".typed-string.email"), Some(Type::TypedString(TypedStringKind::Email)));
        assert_eq!(Type::from_path(".code.javascript"), Some(Type::Code("javascript".to_string())));
        assert_eq!(Type::from_path(".$types.UserType"), Some(Type::TypeRef(KeyCmpValue::String("UserType".to_string()))));
        assert_eq!(Type::from_path(".UserType"), Some(Type::TypeRef(KeyCmpValue::String("UserType".to_string()))));  // Uppercase = type ref
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
        use crate::value_validator::{ValidationError, ValidationErrorKind, Severity};
        
        let type_error = ValidationError {
            kind: ValidationErrorKind::TypeMismatch {
                expected: "string".to_string(),
                actual: "number".to_string(),
            },
            severity: Severity::Error,
            span: None,
        };
        assert_eq!(type_error.severity, Severity::Error);
    }
}
