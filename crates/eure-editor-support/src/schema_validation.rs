//! Schema validation support for EURE editor integration

use eure_schema::{
    DocumentSchema, PathSegment, Severity, ValidationError, ValidationErrorKind,
    document_to_schema, validate_document as validate_with_schema,
};
use eure_tree::tree::LineNumbers;
use eure_tree::{Cst, document::EureDocument};
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
        let schema =
            document_to_schema(&document).map_err(|e| format!("Failed to extract schema: {e}"))?;

        self.schemas.insert(uri.to_string(), schema);
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
    cached_document: Option<&EureDocument>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Create line numbers helper for span conversion
    let line_numbers = LineNumbers::new(input);

    // Try to parse to EureDocument, or use cached if parsing fails
    let using_cached = cached_document.is_some();

    // Either use cached document or parse a new one
    let parsed_document;
    let document = if let Some(cached_doc) = cached_document {
        // Use cached document if provided (typically when there are parse errors)
        cached_doc
    } else {
        // Parse to EureDocument
        let mut visitor = eure_tree::value_visitor::ValueVisitor::new(input);
        if let Err(e) = tree.visit_from_root(&mut visitor) {
            eprintln!("Failed to visit tree: {e}");
            return diagnostics;
        }
        parsed_document = visitor.into_document();
        &parsed_document
    };

    // Add info diagnostic if using cached document
    if using_cached {
        diagnostics.push(Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            severity: Some(DiagnosticSeverity::INFORMATION),
            code: Some(lsp_types::NumberOrString::String(
                "eure-cached-validation".to_string(),
            )),
            code_description: None,
            source: Some("eure-schema".to_string()),
            message: "Schema validation using last valid document structure due to syntax errors"
                .to_string(),
            related_information: None,
            tags: None,
            data: None,
        });
    }

    // Check if there's an external schema to use
    if let Some(schema_uri) = schema_manager.get_document_schema_uri(uri)
        && let Some(schema) = schema_manager.get_schema(schema_uri)
    {
        // Validate against the external schema
        let errors = validate_with_schema(document, schema);
        for error in errors {
            diagnostics.push(validation_error_to_diagnostic(
                &error,
                uri,
                &line_numbers,
                input,
                document,
                tree,
            ));
        }
        return diagnostics;
    }

    // If no external schema, try to extract schema from the document itself
    match document_to_schema(document) {
        Ok(schema) => {
            // Validate document against its own schema
            let errors = validate_with_schema(document, &schema);
            for error in errors {
                diagnostics.push(validation_error_to_diagnostic(
                    &error,
                    uri,
                    &line_numbers,
                    input,
                    document,
                    tree,
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
fn format_field_key(key: &eure_schema::ObjectKey) -> String {
    match key {
        eure_schema::ObjectKey::String(s) => s.clone(),
        eure_schema::ObjectKey::I64(i) => i.to_string(),
        eure_schema::ObjectKey::U64(u) => u.to_string(),
        eure_schema::ObjectKey::Bool(b) => b.to_string(),
        eure_schema::ObjectKey::Null => "null".to_string(),
        eure_schema::ObjectKey::Unit => "()".to_string(),
        eure_schema::ObjectKey::Tuple(elements) => {
            // Format tuple as (elem1, elem2, ...)
            let formatted_elements: Vec<String> = elements.iter().map(format_field_key).collect();
            format!("({})", formatted_elements.join(", "))
        }
        eure_schema::ObjectKey::Hole => "!".to_string(),
    }
}

/// Default range when we can't determine the actual span
fn default_range(_line_numbers: &LineNumbers) -> Range {
    // Default to beginning of document
    Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 0,
        },
    }
}

/// Get the span for a CST node
fn get_node_span(
    cst: &eure_tree::Cst,
    node_id: eure_tree::tree::CstNodeId,
) -> Option<eure_tree::tree::InputSpan> {
    // Get the node data using CstFacade
    let node_data = cst.node_data(node_id)?;

    match node_data {
        eure_tree::tree::CstNodeData::Terminal { data, .. } => match data {
            eure_tree::tree::TerminalData::Input(span) => Some(span),
            eure_tree::tree::TerminalData::Dynamic(_) => None,
        },
        eure_tree::tree::CstNodeData::NonTerminal { data, .. } => {
            match data {
                eure_tree::tree::NonTerminalData::Input(span) => Some(span),
                eure_tree::tree::NonTerminalData::Dynamic => {
                    // For dynamic non-terminals, we need to calculate the span from children
                    calculate_span_from_children(cst, node_id)
                }
            }
        }
    }
}

/// Calculate span from the first and last children of a node
fn calculate_span_from_children(
    cst: &eure_tree::Cst,
    node_id: eure_tree::tree::CstNodeId,
) -> Option<eure_tree::tree::InputSpan> {
    let children: Vec<_> = cst.children(node_id).collect();
    if children.is_empty() {
        return None;
    }

    // Get span of first child
    let first_span = get_node_span(cst, children[0])?;

    // If only one child, return its span
    if children.len() == 1 {
        return Some(first_span);
    }

    // Get span of last child
    let last_span = get_node_span(cst, children[children.len() - 1])?;

    // Merge spans
    Some(first_span.merge(last_span))
}

/// Get a more precise span for value nodes, excluding structural elements
fn get_value_span(
    cst: &eure_tree::Cst,
    node_id: eure_tree::tree::CstNodeId,
) -> Option<eure_tree::tree::InputSpan> {
    // Try to find the actual value content within the node
    // by looking for specific node types that represent values

    // First check if this node is already a terminal (leaf) node
    if let Some(node_data) = cst.node_data(node_id)
        && matches!(node_data, eure_tree::tree::CstNodeData::Terminal { .. })
    {
        return get_node_span(cst, node_id);
    }

    // For non-terminals, try to find the value part
    // This is a heuristic approach - we look for children that are likely to be the actual value
    let children: Vec<_> = cst.children(node_id).collect();

    // Skip leading whitespace/structural nodes and find the actual value
    for child_id in children {
        if let Some(child_data) = cst.node_data(child_id) {
            // Check if this looks like a value node (string, number, etc.)
            if matches!(child_data, eure_tree::tree::CstNodeData::Terminal { .. })
                && let Some(span) = get_node_span(cst, child_id)
            {
                return Some(span);
            }
        }
    }

    // Fall back to the original span
    get_node_span(cst, node_id)
}

/// Convert a ValidationError to an LSP Diagnostic
pub fn validation_error_to_diagnostic(
    error: &ValidationError,
    _uri: &str,
    line_numbers: &LineNumbers,
    _input: &str,
    document: &eure_tree::document::EureDocument,
    cst: &eure_tree::Cst,
) -> Diagnostic {
    // Try to get the actual span from the node
    let range = if let Some(cst_node_id) = document.get_cst_node_id(error.node_id) {
        // Get the span for this CST node
        let span = get_node_span(cst, cst_node_id);

        if let Some(mut span) = span {
            // For certain error types, we might need to refine the span
            // to exclude leading/trailing whitespace
            if let ValidationErrorKind::TypeMismatch { .. } = &error.kind {
                // For type mismatches, try to get a more precise span
                // by looking at the actual value node
                if let Some(refined_span) = get_value_span(cst, cst_node_id) {
                    span = refined_span;
                }
            }

            // Convert span to range
            let start_info = line_numbers.get_char_info(span.start);
            let end_info = line_numbers.get_char_info(span.end);

            // For certain error types, try to refine the position to exclude leading whitespace
            let refined_start = match &error.kind {
                ValidationErrorKind::TypeMismatch { .. } => {
                    // Skip leading whitespace in the span
                    let start_offset = span.start as usize;
                    let end_offset = (span.end as usize).min(_input.len());
                    if start_offset < end_offset {
                        let span_text = &_input[start_offset..end_offset];
                        let trimmed_offset = span_text.len() - span_text.trim_start().len();
                        if trimmed_offset > 0 {
                            line_numbers.get_char_info(span.start + trimmed_offset as u32)
                        } else {
                            start_info
                        }
                    } else {
                        start_info
                    }
                }
                _ => start_info,
            };

            Range {
                start: Position {
                    line: refined_start.line_number,
                    character: refined_start.column_number,
                },
                end: Position {
                    line: end_info.line_number,
                    character: end_info.column_number,
                },
            }
        } else {
            default_range(line_numbers)
        }
    } else {
        default_range(line_numbers)
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
        ValidationErrorKind::StringLengthViolation { min, max, length } => {
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
                        if i + 1 < path.len()
                            && let PathSegment::ArrayIndex(idx) = &path[i + 1]
                        {
                            // Combine identifier with array index
                            if let Some(index) = *idx {
                                path_parts.push(format!("{}[{}]", id.as_ref(), index));
                            } else {
                                path_parts.push(format!("{}[]", id.as_ref()));
                            }
                            i += 2; // Skip the ArrayIndex segment
                            continue;
                        }
                        path_parts.push(id.as_ref().to_string());
                    }
                    PathSegment::Extension(id) => path_parts.push(format!("${}", id.as_ref())),
                    PathSegment::Value(v) => path_parts.push(format!("[{v:?}]")),
                    PathSegment::TupleIndex(idx) => path_parts.push(format!("[{idx}]")),
                    PathSegment::ArrayIndex(idx) => {
                        // Standalone array index (shouldn't normally happen after an ident)
                        if let Some(index) = *idx {
                            path_parts.push(format!("[{index}]"));
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
        ValidationErrorKind::ArrayLengthViolation { min, max, length } => {
            let constraint = match (min, max) {
                (Some(min), Some(max)) => format!("between {min} and {max} items"),
                (Some(min), None) => format!("at least {min} items"),
                (None, Some(max)) => format!("at most {max} items"),
                (None, None) => "unknown".to_string(),
            };
            (
                format!("Array length {length} does not meet constraint: {constraint}"),
                Some("eure-schema-array-length".to_string()),
                None,
            )
        }
        ValidationErrorKind::VariantDiscriminatorMissing => (
            "Variant discriminator field is missing".to_string(),
            Some("eure-schema-variant-discriminator".to_string()),
            None,
        ),
        ValidationErrorKind::MaxDepthExceeded { depth, max_depth } => (
            format!(
                "Maximum validation depth of {max_depth} exceeded at depth {depth} - possible circular reference"
            ),
            Some("eure-schema-max-depth".to_string()),
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
        let path_str = schema_ref
            .strip_prefix("file://")
            .ok_or_else(|| format!("Failed to strip 'file://' prefix from: {}", schema_ref))?;
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
        // Create a test input with valid EURE syntax
        let input = "field1 = \"value1\"\nfield2 = 123\nfield3 = \"value3\"";
        let line_numbers = LineNumbers::new(input);

        let error = ValidationError {
            kind: ValidationErrorKind::TypeMismatch {
                expected: "string".to_string(),
                actual: "number".to_string(),
            },
            severity: Severity::Error,
            node_id: eure_tree::document::NodeId(0),
        };

        // For testing, create a dummy document and CST
        let parse_result = eure_parol::parse_tolerant(input);
        let cst = parse_result.cst();
        let mut visitor = eure_tree::value_visitor::ValueVisitor::new(input);
        let _ = cst.visit_from_root(&mut visitor);
        let document = visitor.into_document();

        let diagnostic = validation_error_to_diagnostic(
            &error,
            "file:///test.eure",
            &line_numbers,
            input,
            &document,
            &cst,
        );

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

        // Check range - NodeId(0) points to the root/first node which is on line 0
        assert_eq!(diagnostic.range.start.line, 0); // Line 1 (0-based)
        assert_eq!(diagnostic.range.start.character, 0); // Column 0 (0-based)
        // The exact end position depends on what NodeId(0) represents, but it should be on line 0
        assert_eq!(diagnostic.range.end.line, 0); // Still on line 1
    }
}
