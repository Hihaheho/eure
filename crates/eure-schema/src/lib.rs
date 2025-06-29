//! EURE Schema validation library
//! 
//! This library provides schema extraction and validation for EURE documents.
//! It supports both standalone schema files and inline schemas within documents.

mod schema;
mod extractor;
mod validator;

pub use schema::*;
pub use extractor::SchemaExtractor;
pub use validator::{SchemaValidator, ValidationError, ValidationErrorKind, Severity};


/// Result of validating a self-describing document
pub struct ValidationResult {
    /// The extracted schema information
    pub schema: ExtractedSchema,
    /// Validation errors found
    pub errors: Vec<ValidationError>,
}

/// Extract schema information from a EURE document
/// 
/// This will extract:
/// - Type definitions from $types namespace
/// - Inline schemas from field extensions
/// - Global settings like cascade types
pub fn extract_schema(input: &str, tree: &eure_tree::Cst) -> ExtractedSchema {
    let mut extractor = SchemaExtractor::new(input);
    // Use the visitor pattern to visit the tree
    use eure_tree::visitor::CstVisitorSuper;
    let root_handle = tree.root_handle();
    extractor.visit_root_handle(root_handle, tree).expect("Schema extraction should not fail");
    extractor.extract()
}

/// Validate a document against a provided schema
/// 
/// # Arguments
/// * `input` - The input string
/// * `tree` - The document to validate
/// * `schema` - The schema to validate against
/// 
/// # Returns
/// A vector of validation errors (may include warnings)
pub fn validate_with_schema(
    input: &str,
    tree: &eure_tree::Cst,
    schema: DocumentSchema,
) -> Vec<ValidationError> {
    SchemaValidator::new(input, schema).validate(tree)
}

/// Extract schema and validate in one pass for self-describing documents
/// 
/// This is useful for documents that contain their own schema definitions.
/// Pure schema documents (containing only $types definitions) are not validated.
/// 
/// # Arguments
/// * `input` - The input string
/// * `tree` - The self-describing document
/// 
/// # Returns
/// The extracted schema and any validation errors
pub fn validate_self_describing(input: &str, tree: &eure_tree::Cst) -> ValidationResult {
    // First pass: extract schema
    let extracted = extract_schema(input, tree);
    
    // Second pass: validate against extracted schema
    let errors = if extracted.is_pure_schema {
        // Pure schema documents don't need validation
        Vec::new()
    } else {
        validate_with_schema(input, tree, extracted.document_schema.clone())
    };
    
    ValidationResult {
        schema: extracted,
        errors,
    }
}

/// Create a validator function from a schema document
/// 
/// This is useful when you want to validate multiple documents against
/// the same schema.
/// 
/// # Arguments
/// * `schema_tree` - A parsed EURE document containing schema definitions
/// 
/// # Returns
/// A function that validates documents against the schema, or an error
/// if the provided document is not a pure schema.
pub fn create_validator(
    input: &str,
    schema_tree: &eure_tree::Cst,
) -> Result<DocumentSchema, String> {
    let extracted = extract_schema(input, schema_tree);
    
    if !extracted.is_pure_schema {
        return Err("Provided document is not a pure schema".to_string());
    }
    
    Ok(extracted.document_schema)
}

/// Check if validation errors contain any errors (not just warnings)
pub fn has_errors(errors: &[ValidationError]) -> bool {
    errors.iter().any(|e| e.severity == Severity::Error)
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
        assert_eq!(Type::from_path(".$types.UserType"), Some(Type::TypeRef("UserType".to_string())));
        assert_eq!(Type::from_path(".UserType"), Some(Type::TypeRef("UserType".to_string())));  // Uppercase = type ref
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
        use crate::validator::{ValidationError, ValidationErrorKind, Severity};
        use eure_tree::tree::InputSpan;
        
        let type_error = ValidationError {
            kind: ValidationErrorKind::TypeMismatch {
                expected: "string".to_string(),
                actual: "number".to_string(),
            },
            span: InputSpan::new(0, 10),
            severity: Severity::Error,
        };
        assert_eq!(type_error.severity, Severity::Error);
        
        let pref_error = ValidationError {
            kind: ValidationErrorKind::PreferSection {
                path: vec!["foo".to_string()],
            },
            span: InputSpan::new(0, 10),
            severity: Severity::Warning,
        };
        assert_eq!(pref_error.severity, Severity::Warning);
    }
}
