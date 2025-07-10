//! Schema validation support for EURE editor integration

use eure_schema::{
    DocumentSchema, PathSegment, ValidationError, ValidationErrorKind, Severity,
    validate_document as validate_with_schema, document_to_schema,
};
use eure_tree::Cst;
use eure_tree::tree::LineNumbers;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
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
    pub fn load_schema(&mut self, uri: &str, input: &str, tree: &Cst) -> Result<(), String> {
        // Parse to EureDocument
        let mut visitor = eure_tree::value_visitor::ValueVisitor::new(input);
        tree.visit_from_root(&mut visitor)
            .map_err(|e| format!("Failed to visit tree: {e}"))?;
        let document = visitor.into_document();
        
        // Extract schema from document
        let schema = document_to_schema(&document)
            .map_err(|e| format!("Failed to extract schema: {e}"))?;

        self.schemas
            .insert(uri.to_string(), schema);
        Ok(())
    }

    /// Get a schema by URI
    pub fn get_schema(&self, uri: &str) -> Option<&DocumentSchema> {
        self.schemas.get(uri)
    }

    /// Associate a document with a schema
    pub fn set_document_schema(&mut self, doc_uri: &str, schema_uri: &str) {
        self.schema_paths
            .insert(doc_uri.to_string(), schema_uri.to_string());
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

    // Parse to EureDocument first
    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(input);
    if let Err(e) = tree.visit_from_root(&mut visitor) {
        eprintln!("Failed to visit tree: {e}");
        return diagnostics;
    }
    let document = visitor.into_document();

    // Check if there's an external schema to use
    if let Some(schema_uri) = schema_manager.get_document_schema_uri(uri)
        && let Some(schema) = schema_manager.get_schema(schema_uri)
    {
        // Validate against the external schema
        let errors = validate_with_schema(&document, schema);
        for error in errors {
            diagnostics.push(validation_error_to_diagnostic(
                &error,
                uri,
                &line_numbers,
                input,
            ));
        }
        return diagnostics;
    }

    // If no external schema, try to extract schema from the document itself
    match document_to_schema(&document) {
        Ok(schema) => {
            // Validate document against its own schema
            let errors = validate_with_schema(&document, &schema);
            for error in errors {
                diagnostics.push(validation_error_to_diagnostic(
                    &error,
                    uri,
                    &line_numbers,
                    input,
                ));
            }
        }
        Err(e) => {
            // Document doesn't define a schema - that's okay
            eprintln!("No schema found in document: {e}");
        }
    }

    diagnostics
}


/// Find a schema file for a document
pub fn find_schema_for_document(doc_path: &Path, workspace_root: Option<&Path>) -> Option<PathBuf> {
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
            && dir == root
        {
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
        eure_schema::KeyCmpValue::Tuple(_) => todo!(),
        eure_schema::KeyCmpValue::MetaExtension(meta) => format!("$${meta}"),
        eure_schema::KeyCmpValue::Hole => "!".to_string(),
    }
}

/// Adjust a span to exclude leading and trailing whitespace
fn trim_span_whitespace(
    span: eure_tree::tree::InputSpan,
    input: &str,
) -> eure_tree::tree::InputSpan {
    let mut start = span.start as usize;
    let mut end = span.end as usize;

    // Safety check
    if start >= input.len() || end > input.len() || start >= end {
        return span;
    }

    // Trim leading whitespace
    let span_text = &input[start..end];
    let leading_ws = span_text.len() - span_text.trim_start().len();
    start += leading_ws;

    // Trim trailing whitespace
    let trimmed_text = &input[start..end];
    let trailing_ws = trimmed_text.len() - trimmed_text.trim_end().len();
    end -= trailing_ws;

    // Ensure we didn't trim everything
    if start >= end {
        return span;
    }

    eure_tree::tree::InputSpan {
        start: start as u32,
        end: end as u32,
    }
}

/// Convert a ValidationError to an LSP Diagnostic
pub fn validation_error_to_diagnostic(
    error: &ValidationError,
    _uri: &str,
    line_numbers: &LineNumbers,
    input: &str,
) -> Diagnostic {
    // For now, we don't have span information from node_id
    // TODO: Pass document to get span from node_id
    let default_info = line_numbers.get_char_info(0);
    let (start_info, end_info) = (default_info, default_info);

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
                format!(
                    "Required field '{}' is missing{}",
                    format_field_key(field),
                    path_str
                ),
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
        ValidationErrorKind::LengthViolation { min, max, length } => {
            let constraint = match (min, max) {
                (Some(min), Some(max)) => format!("between {min} and {max}"),
                (Some(min), None) => format!("at least {min}"),
                (None, Some(max)) => format!("at most {max}"),
                (None, None) => "unknown".to_string(),
            };
            (
                format!("String length {length} does not meet constraint: {constraint}"),
                Some("eure-schema-length".to_string()),
                None,
            )
        }
        ValidationErrorKind::PatternMismatch { pattern, value } => (
            format!("Value '{value}' does not match pattern: {pattern}"),
            Some("eure-schema-pattern".to_string()),
            None,
        ),
        ValidationErrorKind::InvalidValue(msg) if msg.contains("pattern") => (
            format!("Invalid value: {msg}"),
            Some("eure-schema-invalid-pattern".to_string()),
            None,
        ),
        ValidationErrorKind::RangeViolation { min, max, value } => {
            let constraint = match (min, max) {
                (Some(min), Some(max)) => format!("between {min} and {max}"),
                (Some(min), None) => format!("at least {min}"),
                (None, Some(max)) => format!("at most {max}"),
                (None, None) => "unknown".to_string(),
            };
            (
                format!("Number {value} is not {constraint}"),
                Some("eure-schema-range".to_string()),
                None,
            )
        }
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
        ValidationErrorKind::InvalidValue(msg) => (
            format!("Invalid value: {msg}"),
            Some("eure-schema-invalid-value".to_string()),
            None,
        ),
        ValidationErrorKind::HoleExists { path } => {
            let mut path_parts = Vec::new();
            let mut i = 0;
            
            while i < path.len() {
                match &path[i] {
                    PathSegment::Ident(id) => {
                        // Check if next segment is ArrayIndex
                        if i + 1 < path.len() {
                            if let PathSegment::ArrayIndex(idx) = &path[i + 1] {
                                // Combine identifier with array index
                                if let Some(index) = *idx {
                                    path_parts.push(format!("{}[{}]", id.as_ref(), index));
                                } else {
                                    path_parts.push(format!("{}[]", id.as_ref()));
                                }
                                i += 2; // Skip the ArrayIndex segment
                                continue;
                            }
                        }
                        path_parts.push(id.as_ref().to_string());
                    }
                    PathSegment::Extension(id) => path_parts.push(format!("${}", id.as_ref())),
                    PathSegment::MetaExt(id) => path_parts.push(format!("$${}", id.as_ref())),
                    PathSegment::Value(v) => path_parts.push(format!("[{v:?}]")),
                    PathSegment::TupleIndex(idx) => path_parts.push(format!("[{idx}]")),
                    PathSegment::ArrayIndex(idx) => {
                        // Standalone array index (shouldn't normally happen after an ident)
                        if let Some(index) = *idx {
                            path_parts.push(format!("[{}]", index));
                        } else {
                            path_parts.push("[]".to_string());
                        }
                    }
                }
                i += 1;
            }
            
            let path_str = path_parts.join(".");
            (
                format!(
                    "Hole value (!) found at '{path_str}' - holes must be filled with actual values"
                ),
                Some("eure-schema-hole-exists".to_string()),
                None,
            )
        }
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

        let diagnostic =
            validation_error_to_diagnostic(&error, "file:///test.eure", &line_numbers, input);

        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostic.source, Some("eure-schema".to_string()));
        assert_eq!(
            diagnostic.message,
            "Type mismatch: expected string, found number"
        );
        assert_eq!(
            diagnostic.code,
            Some(lsp_types::NumberOrString::String(
                "eure-schema-type".to_string()
            ))
        );

        // Check range (line_numbers returns 0-based positions)
        assert_eq!(diagnostic.range.start.line, 1); // Line 2 (0-based)
        assert_eq!(diagnostic.range.start.character, 6); // Column 6 (0-based)
        assert_eq!(diagnostic.range.end.line, 1); // Line 2
        assert_eq!(diagnostic.range.end.character, 10); // Column 10
    }
}
