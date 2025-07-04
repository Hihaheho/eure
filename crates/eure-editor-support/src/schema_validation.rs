//! Schema validation support for EURE editor integration

use eure_schema::{
    DocumentSchema, Severity, ValidationError, ValidationErrorKind,
    extract_schema_from_value, validate_self_describing_with_tree, validate_with_tree,
};
use eure_tree::Cst;
use eure_tree::tree::LineNumbers;
use lsp_types::{
    Diagnostic, DiagnosticSeverity, Position, Range,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Manages schemas for a workspace
pub struct SchemaManager {
    /// Cached schemas by URI
    schemas: HashMap<String, DocumentSchema>,
    /// Mapping from document URI to schema URI
    schema_paths: HashMap<String, String>,
}

impl Default for SchemaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaManager {
    /// Create a new schema manager
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            schema_paths: HashMap::new(),
        }
    }

    /// Load a schema from a file
    pub fn load_schema(&mut self, uri: &str, input: &str, _tree: &Cst) -> Result<(), String> {
        let extracted = extract_schema_from_value(input)
            .map_err(|e| format!("Failed to extract schema: {}", e))?;
        
        // We don't reject schemas with non-schema content anymore
        // A schema file can contain examples, documentation, etc.
        // The important thing is that it contains schema definitions
        
        self.schemas.insert(uri.to_string(), extracted.document_schema);
        Ok(())
    }

    /// Get a schema by URI
    pub fn get_schema(&self, uri: &str) -> Option<&DocumentSchema> {
        self.schemas.get(uri)
    }

    /// Associate a document with a schema
    pub fn set_document_schema(&mut self, doc_uri: &str, schema_uri: &str) {
        self.schema_paths.insert(doc_uri.to_string(), schema_uri.to_string());
    }

    /// Get the schema URI for a document
    pub fn get_document_schema_uri(&self, doc_uri: &str) -> Option<&str> {
        self.schema_paths.get(doc_uri).map(|s| s.as_str())
    }
}

/// Validate a document with schema support
pub fn validate_document(
    uri: &str,
    input: &str,
    tree: &Cst,
    schema_manager: &SchemaManager,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Create line numbers helper for span conversion
    let line_numbers = LineNumbers::new(input);
    
    // Check if there's an external schema to use
    if let Some(schema_uri) = schema_manager.get_document_schema_uri(uri)
        && let Some(schema) = schema_manager.get_schema(schema_uri) {
            // Validate against the external schema
            match validate_with_tree(input, schema.clone(), tree) {
                Ok(errors) => {
                    for error in errors {
                        diagnostics.push(validation_error_to_diagnostic(&error, uri, &line_numbers));
                    }
                }
                Err(e) => {
                    eprintln!("Validation error: {}", e);
                }
            }
            return diagnostics;
        }
    
    // If no external schema, fall back to self-describing validation
    let validation_result = match validate_self_describing_with_tree(input, tree) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Self-describing validation error: {}", e);
            return diagnostics;
        }
    };
    
    // If it's a pure schema, we don't need to validate it
    if validation_result.schema.is_pure_schema {
        return diagnostics;
    }
    
    // Convert validation errors to diagnostics
    for error in validation_result.errors {
        diagnostics.push(validation_error_to_diagnostic(&error, uri, &line_numbers));
    }

    diagnostics
}

/// Validate a document and return the extracted schema info
pub fn validate_and_extract_schema(
    input: &str,
    tree: &Cst,
) -> Result<eure_schema::ValidationResult, Box<dyn std::error::Error>> {
    validate_self_describing_with_tree(input, tree)
}

/// Find a schema file for a document
pub fn find_schema_for_document(
    doc_path: &Path,
    workspace_root: Option<&Path>,
) -> Option<PathBuf> {
    // Check for .schema.eure file with same base name
    let doc_stem = doc_path.file_stem()?;
    let schema_name = format!("{}.schema.eure", doc_stem.to_str()?);
    
    // 1. Check same directory
    if let Some(parent) = doc_path.parent() {
        let schema_path = parent.join(&schema_name);
        if schema_path.exists() {
            return Some(schema_path);
        }
        
        // 2. Check for generic schema.eure in same directory
        let generic_schema = parent.join("schema.eure");
        if generic_schema.exists() {
            return Some(generic_schema);
        }
    }
    
    // 3. Walk up parent directories
    let mut current = doc_path.parent();
    while let Some(dir) = current {
        let schema_path = dir.join(&schema_name);
        if schema_path.exists() {
            return Some(schema_path);
        }
        
        let generic_schema = dir.join("schema.eure");
        if generic_schema.exists() {
            return Some(generic_schema);
        }
        
        // Stop at workspace root if provided
        if let Some(root) = workspace_root
            && dir == root {
                break;
            }
        
        current = dir.parent();
    }
    
    // 4. Check workspace .eure/schemas directory
    if let Some(root) = workspace_root {
        let schemas_dir = root.join(".eure").join("schemas");
        if schemas_dir.exists() {
            let schema_path = schemas_dir.join(&schema_name);
            if schema_path.exists() {
                return Some(schema_path);
            }
            
            // Check for default.schema.eure
            let default_schema = schemas_dir.join("default.schema.eure");
            if default_schema.exists() {
                return Some(default_schema);
            }
        }
    }
    
    None
}

/// Format a field key for display
fn format_field_key(key: &eure_schema::KeyCmpValue) -> String {
    match key {
        eure_schema::KeyCmpValue::String(s) => s.clone(),
        eure_schema::KeyCmpValue::I64(i) => i.to_string(),
        eure_schema::KeyCmpValue::U64(u) => u.to_string(),
        eure_schema::KeyCmpValue::Bool(b) => b.to_string(),
        eure_schema::KeyCmpValue::Null => "null".to_string(),
        eure_schema::KeyCmpValue::Unit => "()".to_string(),
        eure_schema::KeyCmpValue::Tuple(_) => "<tuple>".to_string(),
        eure_schema::KeyCmpValue::Extension(ext) => format!("${ext}"),
        eure_schema::KeyCmpValue::MetaExtension(meta) => format!("$${meta}"),
    }
}

/// Convert a ValidationError to an LSP Diagnostic
pub fn validation_error_to_diagnostic(error: &ValidationError, _uri: &str, line_numbers: &LineNumbers) -> Diagnostic {
    // Convert byte offsets to line/column positions
    let (start_info, end_info) = if let Some(span) = error.span {
        (line_numbers.get_char_info(span.start), line_numbers.get_char_info(span.end))
    } else {
        // Default to beginning of file if no span
        let default_info = line_numbers.get_char_info(0);
        (default_info, default_info)
    };
    
    let range = Range {
        start: Position {
            line: start_info.line_number,
            character: start_info.column_number,
        },
        end: Position {
            line: end_info.line_number,
            character: end_info.column_number,
        },
    };

    let (message, code, related_info) = match &error.kind {
        ValidationErrorKind::TypeMismatch { expected, actual } => (
            format!("Type mismatch: expected {expected}, found {actual}"),
            Some("eure-schema-type".to_string()),
            None,
        ),
        ValidationErrorKind::UnknownType(type_name) => (
            format!("Unknown type: {type_name}"),
            Some("eure-schema-unknown-type".to_string()),
            None,
        ),
        ValidationErrorKind::RequiredFieldMissing { field, path } => {
            let path_str = if path.is_empty() {
                String::new()
            } else {
                format!(" at {}", eure_schema::path_segments_to_display_string(path))
            };
            (
                format!("Required field '{}' is missing{}", format_field_key(field), path_str),
                Some("eure-schema-required".to_string()),
                None,
            )
        }
        ValidationErrorKind::UnexpectedField { field, path } => {
            let path_str = if path.is_empty() {
                String::new()
            } else {
                format!(" at {}", eure_schema::path_segments_to_display_string(path))
            };
            (
                format!("Unexpected field '{}'{}", format_field_key(field), path_str),
                Some("eure-schema-unexpected".to_string()),
                None,
            )
        }
        ValidationErrorKind::StringLengthViolation { min, max, actual } => {
            let constraint = match (min, max) {
                (Some(min), Some(max)) => format!("between {min} and {max}"),
                (Some(min), None) => format!("at least {min}"),
                (None, Some(max)) => format!("at most {max}"),
                (None, None) => "unknown".to_string(),
            };
            (
                format!("String length {actual} does not meet constraint: {constraint}"),
                Some("eure-schema-length".to_string()),
                None,
            )
        }
        ValidationErrorKind::StringPatternViolation { pattern, value } => (
            format!("Value '{value}' does not match pattern: {pattern}"),
            Some("eure-schema-pattern".to_string()),
            None,
        ),
        ValidationErrorKind::InvalidSchemaPattern { pattern, error } => (
            format!("Invalid regex pattern '{pattern}': {error}"),
            Some("eure-schema-invalid-pattern".to_string()),
            None,
        ),
        ValidationErrorKind::NumberRangeViolation { min, max, actual } => {
            let constraint = match (min, max) {
                (Some(min), Some(max)) => format!("between {min} and {max}"),
                (Some(min), None) => format!("at least {min}"),
                (None, Some(max)) => format!("at most {max}"),
                (None, None) => "unknown".to_string(),
            };
            (
                format!("Number {actual} is not {constraint}"),
                Some("eure-schema-range".to_string()),
                None,
            )
        }
        ValidationErrorKind::ArrayLengthViolation { min, max, actual } => {
            let constraint = match (min, max) {
                (Some(min), Some(max)) => format!("between {min} and {max} items"),
                (Some(min), None) => format!("at least {min} items"),
                (None, Some(max)) => format!("at most {max} items"),
                (None, None) => "unknown".to_string(),
            };
            (
                format!("Array has {actual} items, expected {constraint}"),
                Some("eure-schema-array-length".to_string()),
                None,
            )
        }
        ValidationErrorKind::ArrayUniqueViolation { duplicate } => (
            format!("Array contains duplicate value: {duplicate}"),
            Some("eure-schema-unique".to_string()),
            None,
        ),
        ValidationErrorKind::UnknownVariant { variant, available } => {
            let available_str = if available.is_empty() {
                "none defined".to_string()
            } else {
                available.join(", ")
            };
            (
                format!("Unknown variant '{variant}'. Available variants: {available_str}"),
                Some("eure-schema-variant".to_string()),
                None,
            )
        }
        ValidationErrorKind::MissingVariantTag => (
            "Missing variant tag".to_string(),
            Some("eure-schema-variant-tag".to_string()),
            None,
        ),
        ValidationErrorKind::PreferSection { path } => (
            format!(
                "Consider using section syntax for '{}' instead of inline binding",
                eure_schema::path_segments_to_display_string(path)
            ),
            Some("eure-schema-prefer-section".to_string()),
            None,
        ),
        ValidationErrorKind::PreferArraySyntax { path } => (
            format!(
                "Consider using array syntax [] for '{}' instead of repeated fields",
                eure_schema::path_segments_to_display_string(path)
            ),
            Some("eure-schema-prefer-array".to_string()),
            None,
        ),
        ValidationErrorKind::VariantDiscriminatorMissing => (
            "Variant discriminator field '$variant' is missing".to_string(),
            Some("eure-schema-variant-discriminator".to_string()),
            None,
        ),
        ValidationErrorKind::InvalidVariantDiscriminator(value) => (
            format!("Invalid variant discriminator: {value}"),
            Some("eure-schema-invalid-discriminator".to_string()),
            None,
        ),
        ValidationErrorKind::InvalidValue(msg) => (
            format!("Invalid value: {msg}"),
            Some("eure-schema-invalid-value".to_string()),
            None,
        ),
        ValidationErrorKind::InternalError(msg) => (
            format!("Internal error: {msg}"),
            Some("eure-schema-internal-error".to_string()),
            None,
        ),
    };

    let severity = match error.severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
    };

    Diagnostic {
        range,
        severity: Some(severity),
        code: code.map(lsp_types::NumberOrString::String),
        code_description: None,
        source: Some("eure-schema".to_string()),
        message,
        related_information: related_info,
        tags: None,
        data: None,
    }
}

/// Resolve a schema reference (URL or local path) to a file path
pub fn resolve_schema_reference(
    doc_path: &Path,
    schema_ref: &str,
    _workspace_root: Option<&Path>,
) -> Result<PathBuf, String> {
    if schema_ref.starts_with("http://") || schema_ref.starts_with("https://") {
        // Remote schemas not yet supported
        Err("Remote schemas are not yet supported".to_string())
    } else if schema_ref.starts_with("file://") {
        // Handle file:// URLs
        let path_str = schema_ref.strip_prefix("file://").unwrap();
        let path = Path::new(path_str);
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            Err("file:// URLs must use absolute paths".to_string())
        }
    } else {
        // Resolve relative path
        let base = doc_path
            .parent()
            .ok_or_else(|| "Document has no parent directory".to_string())?;
        let schema_path = base.join(schema_ref);
        
        // Try to canonicalize, but if it fails (file doesn't exist yet), just return the path
        Ok(schema_path.canonicalize().unwrap_or(schema_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_schema_same_directory() {
        // TODO: Add tests with temp directories
    }

    #[test]
    fn test_validation_error_to_diagnostic() {
        use eure_tree::tree::InputSpan;
        
        // Create a test input with specific content at known positions
        let input = "line1\nline2 with error\nline3";
        let line_numbers = LineNumbers::new(input);
        
        let error = ValidationError {
            kind: ValidationErrorKind::TypeMismatch {
                expected: "string".to_string(),
                actual: "number".to_string(),
            },
            // Span from "with" (chars 12-16) on line 2
            span: Some(InputSpan::new(12, 16)),
            severity: Severity::Error,
        };
        
        let diagnostic = validation_error_to_diagnostic(&error, "file:///test.eure", &line_numbers);
        
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostic.source, Some("eure-schema".to_string()));
        assert_eq!(diagnostic.message, "Type mismatch: expected string, found number");
        assert_eq!(diagnostic.code, Some(lsp_types::NumberOrString::String("eure-schema-type".to_string())));
        
        // Check range (line_numbers returns 0-based positions)
        assert_eq!(diagnostic.range.start.line, 1); // Line 2 (0-based)
        assert_eq!(diagnostic.range.start.character, 6); // Column 6 (0-based)
        assert_eq!(diagnostic.range.end.line, 1); // Line 2 
        assert_eq!(diagnostic.range.end.character, 10); // Column 10
    }
}