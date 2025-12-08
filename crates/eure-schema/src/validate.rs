//! Document schema validation
//!
//! # Architecture
//!
//! - `ValidationContext`: Internal state during validation (errors, warnings, path tracking)
//! - `ValidationOutput`: Final result returned to callers
//! - `Validate` trait: Implemented by each schema type
//!
//! # Validation Result
//!
//! - `Ok(())`: Structure matches (errors may be accumulated in context)
//! - `Err(())`: Structure doesn't match (used for union variant discrimination)
//!
//! # Hole Values
//!
//! The hole value (`!`) represents an unfilled placeholder:
//! - Type checking: Holes match any schema (always pass)
//! - Completeness: Documents containing holes are valid but not complete

use std::collections::HashSet;

use crate::{
    ArraySchema, Bound, FloatSchema, IntegerSchema, MapSchema, RecordSchema, SchemaDocument,
    SchemaNodeContent, SchemaNodeId, TextSchema, TupleSchema, TypeReference, UnionSchema,
    UnknownFieldsPolicy, identifiers,
};
use eure_document::document::node::{Node, NodeValue};
use eure_document::document::{EureDocument, NodeId};
use eure_document::identifier::Identifier;
use eure_document::parse::VariantPath;
use eure_document::path::{EurePath, PathSegment};
use eure_document::text::Language;
use eure_document::value::{ObjectKey, PrimitiveValue};
use num_bigint::BigInt;
use thiserror::Error;

// =============================================================================
// Public API
// =============================================================================

/// Validate a document against a schema.
///
/// # Example
///
/// ```ignore
/// let output = validate(&document, &schema);
/// if output.is_valid {
///     println!("Document is valid!");
/// } else {
///     for error in &output.errors {
///         println!("Error: {}", error);
///     }
/// }
/// ```
pub fn validate(document: &EureDocument, schema: &SchemaDocument) -> ValidationOutput {
    let root_id = document.get_root_id();
    let mut ctx = ValidationContext::new(document, schema, root_id, schema.root);
    let _ = ctx.validate_node(root_id, schema.root);
    ctx.finish()
}

/// Validate a specific node against a schema node.
pub fn validate_node(
    document: &EureDocument,
    schema: &SchemaDocument,
    node_id: NodeId,
    schema_id: SchemaNodeId,
) -> ValidationOutput {
    let mut ctx = ValidationContext::new(document, schema, node_id, schema_id);
    let _ = ctx.validate_node(node_id, schema_id);
    ctx.finish()
}

// =============================================================================
// ValidationOutput (final result for public API)
// =============================================================================

/// Final validation output returned to callers.
#[derive(Debug, Clone, Default)]
pub struct ValidationOutput {
    /// No type errors (holes are allowed)
    pub is_valid: bool,
    /// No type errors AND no holes
    pub is_complete: bool,
    /// Type errors encountered during validation
    pub errors: Vec<ValidationError>,
    /// Warnings (e.g., unknown extensions)
    pub warnings: Vec<ValidationWarning>,
}

// =============================================================================
// ValidationContext (internal state)
// =============================================================================

/// Internal validation context - manages state during validation.
pub struct ValidationContext<'a> {
    schema: &'a SchemaDocument,
    document: &'a EureDocument,
    path: EurePath,
    has_holes: bool,
    current_node_id: NodeId,
    current_schema_node_id: SchemaNodeId,
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationWarning>,
}

impl<'a> ValidationContext<'a> {
    fn new(
        document: &'a EureDocument,
        schema: &'a SchemaDocument,
        root_node_id: NodeId,
        root_schema_id: SchemaNodeId,
    ) -> Self {
        Self {
            schema,
            document,
            path: EurePath::root(),
            has_holes: false,
            current_node_id: root_node_id,
            current_schema_node_id: root_schema_id,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Record an error (validation continues).
    pub fn record_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Record a warning.
    pub fn record_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Mark that a hole was encountered.
    pub fn mark_has_holes(&mut self) {
        self.has_holes = true;
    }

    /// Fork context for trial validation (union variant testing).
    pub fn fork(&self) -> Self {
        Self {
            schema: self.schema,
            document: self.document,
            path: self.path.clone(),
            has_holes: self.has_holes,
            current_node_id: self.current_node_id,
            current_schema_node_id: self.current_schema_node_id,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Merge results from a forked context.
    pub fn merge(&mut self, other: Self) {
        self.has_holes |= other.has_holes;
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }

    /// Consume context and produce final output.
    fn finish(self) -> ValidationOutput {
        ValidationOutput {
            is_valid: self.errors.is_empty(),
            is_complete: self.errors.is_empty() && !self.has_holes,
            errors: self.errors,
            warnings: self.warnings,
        }
    }

    // -------------------------------------------------------------------------
    // Path management
    // -------------------------------------------------------------------------

    fn node_id(&self) -> NodeId {
        self.current_node_id
    }

    fn schema_node_id(&self) -> SchemaNodeId {
        self.current_schema_node_id
    }

    fn push_path_ident(&mut self, ident: Identifier) {
        self.path.0.push(PathSegment::Ident(ident));
    }

    fn push_path_key(&mut self, key: ObjectKey) {
        self.path.0.push(PathSegment::Value(key));
    }

    fn push_path_index(&mut self, index: usize) {
        self.path.0.push(PathSegment::ArrayIndex(Some(index)));
    }

    fn push_path_tuple_index(&mut self, index: u8) {
        self.path.0.push(PathSegment::TupleIndex(index));
    }

    fn push_path_extension(&mut self, ident: Identifier) {
        self.path.0.push(PathSegment::Extension(ident));
    }

    fn pop_path(&mut self) {
        self.path.0.pop();
    }

    // -------------------------------------------------------------------------
    // Core validation dispatch
    // -------------------------------------------------------------------------

    /// Validate a node against a schema node.
    fn validate_node(&mut self, node_id: NodeId, schema_id: SchemaNodeId) -> Result<(), ()> {
        self.current_node_id = node_id;
        self.current_schema_node_id = schema_id;

        let node = self.document.node(node_id);

        // Check for hole
        if matches!(&node.content, NodeValue::Hole(_)) {
            self.mark_has_holes();
            return Ok(()); // Holes match any schema
        }

        let schema_node = self.schema.node(schema_id);
        let result = self.validate_content(node, &schema_node.content, schema_id);

        // Validate extensions
        self.validate_extensions(node, schema_id);

        result
    }

    fn validate_content(
        &mut self,
        node: &Node,
        content: &SchemaNodeContent,
        _schema_id: SchemaNodeId,
    ) -> Result<(), ()> {
        match content {
            SchemaNodeContent::Any => Ok(()),
            SchemaNodeContent::Text(schema) => self.validate_text(node, schema),
            SchemaNodeContent::Integer(schema) => self.validate_integer(node, schema),
            SchemaNodeContent::Float(schema) => self.validate_float(node, schema),
            SchemaNodeContent::Boolean => self.validate_boolean(node),
            SchemaNodeContent::Null => self.validate_null(node),
            SchemaNodeContent::Literal(expected) => self.validate_literal(node, expected),
            SchemaNodeContent::Array(schema) => self.validate_array(node, schema),
            SchemaNodeContent::Map(schema) => self.validate_map(node, schema),
            SchemaNodeContent::Record(schema) => self.validate_record(node, schema),
            SchemaNodeContent::Tuple(schema) => self.validate_tuple(node, schema),
            SchemaNodeContent::Union(schema) => self.validate_union(node, schema),
            SchemaNodeContent::Reference(type_ref) => self.validate_reference(node, type_ref),
        }
    }

    fn validate_extensions(&mut self, node: &Node, schema_id: SchemaNodeId) {
        let schema_node = self.schema.node(schema_id);
        let ext_types = &schema_node.ext_types;

        // Check for missing required extensions
        for (ext_ident, ext_schema) in ext_types {
            if !ext_schema.optional && !node.extensions.contains_key(ext_ident) {
                self.record_error(ValidationError::MissingRequiredExtension {
                    extension: ext_ident.to_string(),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        }

        // Validate present extensions
        for (ext_ident, &ext_node_id) in &node.extensions {
            // Skip $variant (used for union discrimination)
            if ext_ident == &identifiers::VARIANT {
                continue;
            }

            if let Some(ext_schema) = ext_types.get(ext_ident) {
                self.push_path_extension(ext_ident.clone());
                let _ = self.validate_node(ext_node_id, ext_schema.schema);
                self.pop_path();
            } else {
                self.record_warning(ValidationWarning::UnknownExtension {
                    name: ext_ident.to_string(),
                    path: self.path.clone(),
                });
            }
        }
    }

    // -------------------------------------------------------------------------
    // Primitive type validation
    // -------------------------------------------------------------------------

    fn validate_text(&mut self, node: &Node, schema: &TextSchema) -> Result<(), ()> {
        let text = match &node.content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => t,
            _ => {
                self.record_error(ValidationError::TypeMismatch {
                    expected: "text".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                return Err(());
            }
        };

        // Validate language
        if let Some(expected_lang) = &schema.language {
            let actual_lang = &text.language;
            let matches = match actual_lang {
                Language::Plaintext => expected_lang == "plaintext" || expected_lang == "text",
                Language::Implicit => true, // Implicit can match any
                Language::Other(lang) => lang == expected_lang,
            };
            if !matches {
                self.record_error(ValidationError::LanguageMismatch {
                    expected: expected_lang.clone(),
                    actual: format!("{:?}", actual_lang),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        }

        // Validate min length
        let len = text.as_str().chars().count();
        if let Some(min) = schema.min_length
            && len < min as usize
        {
            self.record_error(ValidationError::StringLengthOutOfBounds {
                length: len,
                min: Some(min),
                max: schema.max_length,
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate max length
        if let Some(max) = schema.max_length
            && len > max as usize
        {
            self.record_error(ValidationError::StringLengthOutOfBounds {
                length: len,
                min: schema.min_length,
                max: Some(max),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate pattern (pre-compiled at parse time)
        if let Some(regex) = &schema.pattern
            && !regex.is_match(text.as_str())
        {
            self.record_error(ValidationError::PatternMismatch {
                pattern: regex.as_str().to_string(),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        Ok(())
    }

    fn validate_integer(&mut self, node: &Node, schema: &IntegerSchema) -> Result<(), ()> {
        let int_val = match &node.content {
            NodeValue::Primitive(PrimitiveValue::Integer(i)) => i,
            _ => {
                self.record_error(ValidationError::TypeMismatch {
                    expected: "integer".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                return Err(());
            }
        };

        // Validate min bound
        let in_range = match (&schema.min, &schema.max) {
            (Bound::Unbounded, Bound::Unbounded) => true,
            (Bound::Inclusive(min), Bound::Unbounded) => int_val >= min,
            (Bound::Exclusive(min), Bound::Unbounded) => int_val > min,
            (Bound::Unbounded, Bound::Inclusive(max)) => int_val <= max,
            (Bound::Unbounded, Bound::Exclusive(max)) => int_val < max,
            (Bound::Inclusive(min), Bound::Inclusive(max)) => int_val >= min && int_val <= max,
            (Bound::Inclusive(min), Bound::Exclusive(max)) => int_val >= min && int_val < max,
            (Bound::Exclusive(min), Bound::Inclusive(max)) => int_val > min && int_val <= max,
            (Bound::Exclusive(min), Bound::Exclusive(max)) => int_val > min && int_val < max,
        };

        if !in_range {
            self.record_error(ValidationError::OutOfRange {
                value: int_val.to_string(),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate multiple-of
        if let Some(divisor) = &schema.multiple_of
            && int_val % divisor != BigInt::from(0)
        {
            self.record_error(ValidationError::NotMultipleOf {
                divisor: divisor.to_string(),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        Ok(())
    }

    fn validate_float(&mut self, node: &Node, schema: &FloatSchema) -> Result<(), ()> {
        let float_val = match &node.content {
            NodeValue::Primitive(PrimitiveValue::F64(f)) => *f,
            NodeValue::Primitive(PrimitiveValue::F32(f)) => *f as f64,
            NodeValue::Primitive(PrimitiveValue::Integer(i)) => {
                // Allow integer to be coerced to float
                i.to_string().parse::<f64>().unwrap_or(f64::NAN)
            }
            _ => {
                self.record_error(ValidationError::TypeMismatch {
                    expected: "float".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                return Err(());
            }
        };

        // Validate bounds
        let in_range = match (&schema.min, &schema.max) {
            (Bound::Unbounded, Bound::Unbounded) => true,
            (Bound::Inclusive(min), Bound::Unbounded) => float_val >= *min,
            (Bound::Exclusive(min), Bound::Unbounded) => float_val > *min,
            (Bound::Unbounded, Bound::Inclusive(max)) => float_val <= *max,
            (Bound::Unbounded, Bound::Exclusive(max)) => float_val < *max,
            (Bound::Inclusive(min), Bound::Inclusive(max)) => {
                float_val >= *min && float_val <= *max
            }
            (Bound::Inclusive(min), Bound::Exclusive(max)) => float_val >= *min && float_val < *max,
            (Bound::Exclusive(min), Bound::Inclusive(max)) => float_val > *min && float_val <= *max,
            (Bound::Exclusive(min), Bound::Exclusive(max)) => float_val > *min && float_val < *max,
        };

        if !in_range {
            self.record_error(ValidationError::OutOfRange {
                value: float_val.to_string(),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate multiple-of
        if let Some(divisor) = schema.multiple_of
            && (float_val % divisor).abs() > f64::EPSILON
        {
            self.record_error(ValidationError::NotMultipleOf {
                divisor: divisor.to_string(),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        Ok(())
    }

    fn validate_boolean(&mut self, node: &Node) -> Result<(), ()> {
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::Bool(_)) => Ok(()),
            _ => {
                self.record_error(ValidationError::TypeMismatch {
                    expected: "boolean".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                Err(())
            }
        }
    }

    fn validate_null(&mut self, node: &Node) -> Result<(), ()> {
        match &node.content {
            NodeValue::Primitive(PrimitiveValue::Null) => Ok(()),
            _ => {
                self.record_error(ValidationError::TypeMismatch {
                    expected: "null".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                Err(())
            }
        }
    }

    fn validate_literal(&mut self, _node: &Node, expected: &EureDocument) -> Result<(), ()> {
        let node_id = self.node_id();
        let actual = node_subtree_to_document(self.document, node_id);
        if actual != *expected {
            self.record_error(ValidationError::LiteralMismatch {
                expected: format!("{:?}", expected),
                actual: format!("{:?}", actual),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
            return Err(());
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Container type validation
    // -------------------------------------------------------------------------

    fn validate_array(&mut self, node: &Node, schema: &ArraySchema) -> Result<(), ()> {
        let arr = match &node.content {
            NodeValue::Array(a) => a,
            _ => {
                self.record_error(ValidationError::TypeMismatch {
                    expected: "array".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                return Err(());
            }
        };

        let len = arr.len();

        // Validate length constraints
        if let Some(min) = schema.min_length
            && len < min as usize
        {
            self.record_error(ValidationError::ArrayLengthOutOfBounds {
                length: len,
                min: Some(min),
                max: schema.max_length,
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }
        if let Some(max) = schema.max_length
            && len > max as usize
        {
            self.record_error(ValidationError::ArrayLengthOutOfBounds {
                length: len,
                min: schema.min_length,
                max: Some(max),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate each item
        for (i, &item_id) in arr.iter().enumerate() {
            self.push_path_index(i);
            let _ = self.validate_node(item_id, schema.item);
            self.pop_path();
        }

        // Validate unique constraint
        if schema.unique {
            self.validate_array_unique(arr);
        }

        // Validate contains constraint
        if let Some(contains_schema) = schema.contains {
            self.validate_array_contains(arr, contains_schema);
        }

        Ok(())
    }

    fn validate_array_unique(&mut self, arr: &eure_document::document::node::NodeArray) {
        // O(n²) comparison - could be optimized with hashing
        let items: Vec<_> = arr.iter().copied().collect();
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                let doc_i = node_subtree_to_document(self.document, items[i]);
                let doc_j = node_subtree_to_document(self.document, items[j]);
                if doc_i == doc_j {
                    self.record_error(ValidationError::ArrayNotUnique {
                        path: self.path.clone(),
                        node_id: self.node_id(),
                        schema_node_id: self.schema_node_id(),
                    });
                    return;
                }
            }
        }
    }

    fn validate_array_contains(
        &mut self,
        arr: &eure_document::document::node::NodeArray,
        contains_schema: SchemaNodeId,
    ) {
        for &item_id in arr.iter() {
            let mut trial_ctx = self.fork();
            if trial_ctx.validate_node(item_id, contains_schema).is_ok()
                && trial_ctx.errors.is_empty()
            {
                return; // Found a matching element
            }
        }
        self.record_error(ValidationError::ArrayMissingContains {
            path: self.path.clone(),
            node_id: self.node_id(),
            schema_node_id: self.schema_node_id(),
        });
    }

    fn validate_map(&mut self, node: &Node, schema: &MapSchema) -> Result<(), ()> {
        let map = match &node.content {
            NodeValue::Map(m) => m,
            _ => {
                self.record_error(ValidationError::TypeMismatch {
                    expected: "map".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                return Err(());
            }
        };

        let size = map.len();

        // Validate size constraints
        if let Some(min) = schema.min_size
            && size < min as usize
        {
            self.record_error(ValidationError::MapSizeOutOfBounds {
                size,
                min: Some(min),
                max: schema.max_size,
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }
        if let Some(max) = schema.max_size
            && size > max as usize
        {
            self.record_error(ValidationError::MapSizeOutOfBounds {
                size,
                min: schema.min_size,
                max: Some(max),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
        }

        // Validate each entry
        for (key, &val_id) in map.iter() {
            // Validate key
            self.validate_object_key(key, schema.key);

            // Validate value
            self.push_path_key(key.clone());
            let _ = self.validate_node(val_id, schema.value);
            self.pop_path();
        }

        Ok(())
    }

    fn validate_object_key(&mut self, key: &ObjectKey, key_schema: SchemaNodeId) {
        let schema_content = self.resolve_schema_content(key_schema);
        match (key, schema_content) {
            (ObjectKey::String(_), SchemaNodeContent::Text(_)) => {}
            (ObjectKey::Number(_), SchemaNodeContent::Integer(_)) => {}
            (ObjectKey::Bool(_), SchemaNodeContent::Boolean) => {}
            (ObjectKey::Tuple(_), SchemaNodeContent::Tuple(_)) => {}
            (_, SchemaNodeContent::Any) => {} // Any accepts any key type
            _ => {
                self.record_error(ValidationError::InvalidKeyType {
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
            }
        }
    }

    /// Resolve type references to get the actual schema content.
    fn resolve_schema_content(&self, schema_id: SchemaNodeId) -> &SchemaNodeContent {
        let mut current_id = schema_id;
        // Limit iterations to prevent infinite loops with circular refs
        for _ in 0..100 {
            let content = &self.schema.node(current_id).content;
            match content {
                SchemaNodeContent::Reference(type_ref) => {
                    if type_ref.namespace.is_some() {
                        return content; // Cross-schema refs not resolved
                    }
                    if let Some(&resolved_id) = self.schema.types.get(&type_ref.name) {
                        current_id = resolved_id;
                    } else {
                        return content; // Unresolved reference
                    }
                }
                _ => return content,
            }
        }
        &self.schema.node(current_id).content
    }

    fn validate_record(&mut self, node: &Node, schema: &RecordSchema) -> Result<(), ()> {
        // Save record-level node IDs for structure errors (missing fields, unknown fields)
        // before recursive validation calls overwrite current_node_id/current_schema_node_id
        let record_node_id = self.node_id();
        let record_schema_node_id = self.schema_node_id();

        // Use RecordParser from eure-document
        let mut rec = match self.document.parse_record(record_node_id) {
            Ok(r) => r,
            Err(_) => {
                self.record_error(ValidationError::TypeMismatch {
                    expected: "record".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.path.clone(),
                    node_id: record_node_id,
                    schema_node_id: record_schema_node_id,
                });
                return Err(());
            }
        };

        // Validate each field in schema
        for (field_name, field_schema) in &schema.properties {
            match rec.field_optional(field_name) {
                Some(field_ctx) => {
                    // Check deprecated
                    let field_schema_node = self.schema.node(field_schema.schema);
                    if field_schema_node.metadata.deprecated {
                        self.record_warning(ValidationWarning::DeprecatedField {
                            field: field_name.clone(),
                            path: self.path.clone(),
                        });
                    }

                    // Validate field value
                    if let Ok(ident) = field_name.parse::<Identifier>() {
                        self.push_path_ident(ident);
                    } else {
                        self.push_path_key(ObjectKey::String(field_name.clone()));
                    }
                    let _ = self.validate_node(field_ctx.node_id(), field_schema.schema);
                    self.pop_path();
                }
                None if !field_schema.optional => {
                    self.record_error(ValidationError::MissingRequiredField {
                        field: field_name.clone(),
                        path: self.path.clone(),
                        node_id: record_node_id,
                        schema_node_id: record_schema_node_id,
                    });
                }
                None => {}
            }
        }

        // Handle unknown fields
        for (field_name, field_ctx) in rec.unknown_fields() {
            match &schema.unknown_fields {
                UnknownFieldsPolicy::Deny => {
                    self.record_error(ValidationError::UnknownField {
                        field: field_name.to_string(),
                        path: self.path.clone(),
                        node_id: record_node_id,
                        schema_node_id: record_schema_node_id,
                    });
                }
                UnknownFieldsPolicy::Allow => {}
                UnknownFieldsPolicy::Schema(s) => {
                    if let Ok(ident) = field_name.parse::<Identifier>() {
                        self.push_path_ident(ident);
                    } else {
                        self.push_path_key(ObjectKey::String(field_name.to_string()));
                    }
                    let _ = self.validate_node(field_ctx.node_id(), *s);
                    self.pop_path();
                }
            }
        }

        Ok(())
    }

    fn validate_tuple(&mut self, node: &Node, schema: &TupleSchema) -> Result<(), ()> {
        let tuple = match &node.content {
            NodeValue::Tuple(t) => t,
            _ => {
                self.record_error(ValidationError::TypeMismatch {
                    expected: "tuple".to_string(),
                    actual: node_type_name(&node.content),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                return Err(());
            }
        };

        // Check length
        if tuple.len() != schema.elements.len() {
            self.record_error(ValidationError::TupleLengthMismatch {
                expected: schema.elements.len(),
                actual: tuple.len(),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
            return Err(());
        }

        // Validate each element
        for (i, (&item_id, &elem_schema)) in tuple.iter().zip(schema.elements.iter()).enumerate() {
            self.push_path_tuple_index(i as u8);
            let _ = self.validate_node(item_id, elem_schema);
            self.pop_path();
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Union validation (simplified: $variant + untagged only)
    // -------------------------------------------------------------------------

    fn validate_union(&mut self, node: &Node, schema: &UnionSchema) -> Result<(), ()> {
        // Check for $variant extension
        if let Some(tag_str) = self.get_extension_as_string(node, &identifiers::VARIANT) {
            match VariantPath::parse(&tag_str) {
                Ok(path) => return self.validate_union_tagged(node, schema, &path),
                Err(_) => {
                    self.record_error(ValidationError::InvalidVariantTag {
                        tag: tag_str,
                        path: self.path.clone(),
                        node_id: self.node_id(),
                        schema_node_id: self.schema_node_id(),
                    });
                    return Ok(());
                }
            }
        }

        // No $variant - try untagged matching
        self.validate_union_untagged(node, schema)
    }

    fn validate_union_tagged(
        &mut self,
        _node: &Node,
        schema: &UnionSchema,
        variant_path: &VariantPath,
    ) -> Result<(), ()> {
        let first = match variant_path.first() {
            Some(f) => f.as_ref(),
            None => {
                self.record_error(ValidationError::InvalidVariantTag {
                    tag: variant_path.to_string(),
                    path: self.path.clone(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                return Ok(());
            }
        };

        if let Some(&variant_schema_id) = schema.variants.get(first) {
            if let Some(rest) = variant_path.rest() {
                // More path segments - variant must be a union
                let nested_schema = {
                    let content = self.resolve_schema_content(variant_schema_id);
                    match content {
                        SchemaNodeContent::Union(s) => Some(s.clone()),
                        _ => None,
                    }
                };

                if let Some(nested) = nested_schema {
                    return self.validate_union_tagged(_node, &nested, &rest);
                } else {
                    self.record_error(ValidationError::InvalidVariantTag {
                        tag: variant_path.to_string(),
                        path: self.path.clone(),
                        node_id: self.node_id(),
                        schema_node_id: self.schema_node_id(),
                    });
                    return Ok(());
                }
            }

            let node_id = self.current_node_id;
            self.validate_node(node_id, variant_schema_id)
        } else {
            self.record_error(ValidationError::InvalidVariantTag {
                tag: variant_path.to_string(),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
            Ok(())
        }
    }

    fn validate_union_untagged(&mut self, _node: &Node, schema: &UnionSchema) -> Result<(), ()> {
        let priority_names: HashSet<_> = schema.priority.iter().flatten().cloned().collect();

        let node_id = self.current_node_id;

        // 1. Try priority variants first (first match wins)
        if let Some(priority) = &schema.priority {
            for name in priority {
                if let Some(&variant_schema) = schema.variants.get(name) {
                    let mut trial_ctx = self.fork();
                    if trial_ctx.validate_node(node_id, variant_schema).is_ok()
                        && trial_ctx.errors.is_empty()
                    {
                        self.merge(trial_ctx);
                        return Ok(());
                    }
                }
            }
        }

        // 2. Try fallback variants (multiple matches = ambiguous)
        let mut matched: Vec<(String, ValidationContext)> = Vec::new();
        for (name, &variant_schema) in &schema.variants {
            if priority_names.contains(name) {
                continue;
            }
            let mut trial_ctx = self.fork();
            if trial_ctx.validate_node(node_id, variant_schema).is_ok()
                && trial_ctx.errors.is_empty()
            {
                matched.push((name.clone(), trial_ctx));
            }
        }

        match matched.len() {
            0 => {
                self.record_error(ValidationError::NoVariantMatched {
                    path: self.path.clone(),
                    variant_errors: Vec::new(),
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                Ok(())
            }
            1 => {
                let (_, trial_ctx) = matched.into_iter().next().unwrap();
                self.merge(trial_ctx);
                Ok(())
            }
            _ => {
                let names: Vec<_> = matched.into_iter().map(|(n, _)| n).collect();
                self.record_error(ValidationError::AmbiguousUnion {
                    path: self.path.clone(),
                    variants: names,
                    node_id: self.node_id(),
                    schema_node_id: self.schema_node_id(),
                });
                Ok(())
            }
        }
    }

    fn get_extension_as_string(&self, node: &Node, ident: &Identifier) -> Option<String> {
        let ext_node_id = node.extensions.get(ident)?;
        let ext_node = self.document.node(*ext_node_id);
        match &ext_node.content {
            NodeValue::Primitive(PrimitiveValue::Text(t)) => Some(t.as_str().to_string()),
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    // Type reference validation
    // -------------------------------------------------------------------------

    fn validate_reference(&mut self, node: &Node, type_ref: &TypeReference) -> Result<(), ()> {
        if type_ref.namespace.is_some() {
            self.record_error(ValidationError::UndefinedTypeReference {
                name: format!("{}.{}", type_ref.namespace.as_ref().unwrap(), type_ref.name),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
            return Ok(());
        }

        if let Some(&schema_id) = self.schema.types.get(&type_ref.name) {
            self.validate_content(node, &self.schema.node(schema_id).content, schema_id)
        } else {
            self.record_error(ValidationError::UndefinedTypeReference {
                name: type_ref.name.to_string(),
                path: self.path.clone(),
                node_id: self.node_id(),
                schema_node_id: self.schema_node_id(),
            });
            Ok(())
        }
    }
}

// =============================================================================
// ValidationError
// =============================================================================

#[derive(Debug, Clone, Error, PartialEq)]
pub enum ValidationError {
    #[error("Type mismatch: expected {expected}, got {actual} at path {path}")]
    TypeMismatch {
        expected: String,
        actual: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Missing required field '{field}' at path {path}")]
    MissingRequiredField {
        field: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Unknown field '{field}' at path {path}")]
    UnknownField {
        field: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Value {value} is out of range at path {path}")]
    OutOfRange {
        value: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("String length {length} is out of bounds at path {path}")]
    StringLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("String does not match pattern '{pattern}' at path {path}")]
    PatternMismatch {
        pattern: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Array length {length} is out of bounds at path {path}")]
    ArrayLengthOutOfBounds {
        length: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Map size {size} is out of bounds at path {path}")]
    MapSizeOutOfBounds {
        size: usize,
        min: Option<u32>,
        max: Option<u32>,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Tuple length mismatch: expected {expected}, got {actual} at path {path}")]
    TupleLengthMismatch {
        expected: usize,
        actual: usize,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Array elements must be unique at path {path}")]
    ArrayNotUnique {
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Array must contain required element at path {path}")]
    ArrayMissingContains {
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("No variant matched for union at path {path}")]
    NoVariantMatched {
        path: EurePath,
        variant_errors: Vec<(String, ValidationError)>,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Multiple variants matched for union at path {path}: {variants:?}")]
    AmbiguousUnion {
        path: EurePath,
        variants: Vec<String>,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Invalid variant tag '{tag}' at path {path}")]
    InvalidVariantTag {
        tag: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Literal value mismatch at path {path}")]
    LiteralMismatch {
        expected: String,
        actual: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Language mismatch: expected {expected}, got {actual} at path {path}")]
    LanguageMismatch {
        expected: String,
        actual: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Invalid key type at path {path}")]
    InvalidKeyType {
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Integer not a multiple of {divisor} at path {path}")]
    NotMultipleOf {
        divisor: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Undefined type reference '{name}' at path {path}")]
    UndefinedTypeReference {
        name: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },

    #[error("Missing required extension '{extension}' at path {path}")]
    MissingRequiredExtension {
        extension: String,
        path: EurePath,
        node_id: NodeId,
        schema_node_id: SchemaNodeId,
    },
}

impl ValidationError {
    pub fn node_ids(&self) -> (NodeId, SchemaNodeId) {
        match self {
            Self::TypeMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::MissingRequiredField {
                node_id,
                schema_node_id,
                ..
            }
            | Self::UnknownField {
                node_id,
                schema_node_id,
                ..
            }
            | Self::OutOfRange {
                node_id,
                schema_node_id,
                ..
            }
            | Self::StringLengthOutOfBounds {
                node_id,
                schema_node_id,
                ..
            }
            | Self::PatternMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::ArrayLengthOutOfBounds {
                node_id,
                schema_node_id,
                ..
            }
            | Self::MapSizeOutOfBounds {
                node_id,
                schema_node_id,
                ..
            }
            | Self::TupleLengthMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::ArrayNotUnique {
                node_id,
                schema_node_id,
                ..
            }
            | Self::ArrayMissingContains {
                node_id,
                schema_node_id,
                ..
            }
            | Self::NoVariantMatched {
                node_id,
                schema_node_id,
                ..
            }
            | Self::AmbiguousUnion {
                node_id,
                schema_node_id,
                ..
            }
            | Self::InvalidVariantTag {
                node_id,
                schema_node_id,
                ..
            }
            | Self::LiteralMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::LanguageMismatch {
                node_id,
                schema_node_id,
                ..
            }
            | Self::InvalidKeyType {
                node_id,
                schema_node_id,
                ..
            }
            | Self::NotMultipleOf {
                node_id,
                schema_node_id,
                ..
            }
            | Self::UndefinedTypeReference {
                node_id,
                schema_node_id,
                ..
            }
            | Self::MissingRequiredExtension {
                node_id,
                schema_node_id,
                ..
            } => (*node_id, *schema_node_id),
        }
    }
}

// =============================================================================
// ValidationWarning
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarning {
    UnknownExtension { name: String, path: EurePath },
    DeprecatedField { field: String, path: EurePath },
}

// =============================================================================
// Helper Functions
// =============================================================================

fn node_type_name(content: &NodeValue) -> String {
    match content {
        NodeValue::Hole(_) => "hole".to_string(),
        NodeValue::Primitive(p) => match p {
            PrimitiveValue::Null => "null".to_string(),
            PrimitiveValue::Bool(_) => "boolean".to_string(),
            PrimitiveValue::Integer(_) => "integer".to_string(),
            PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => "float".to_string(),
            PrimitiveValue::Text(_) => "text".to_string(),
        },
        NodeValue::Array(_) => "array".to_string(),
        NodeValue::Tuple(_) => "tuple".to_string(),
        NodeValue::Map(_) => "map".to_string(),
    }
}

fn node_subtree_to_document(doc: &EureDocument, node_id: NodeId) -> EureDocument {
    let mut result = EureDocument::new();
    let root_id = result.get_root_id();
    copy_subtree(doc, node_id, &mut result, root_id);
    result
}

fn copy_subtree(src: &EureDocument, src_id: NodeId, dst: &mut EureDocument, dst_id: NodeId) {
    let src_node = src.node(src_id);
    dst.node_mut(dst_id).content = src_node.content.clone();

    // Copy extensions (except $variant which is metadata for union discrimination)
    for (ext_ident, &ext_src_id) in &src_node.extensions {
        // Skip $variant extension as it's union metadata, not actual data
        if ext_ident == &identifiers::VARIANT {
            continue;
        }
        if let Ok(result) = dst.add_extension(ext_ident.clone(), dst_id) {
            let child_dst_id = result.node_id;
            copy_subtree(src, ext_src_id, dst, child_dst_id);
        }
    }

    // Copy children based on content type
    match &src_node.content {
        NodeValue::Array(arr) => {
            for &child_src_id in arr.iter() {
                if let Ok(result) = dst.add_array_element(None, dst_id) {
                    let child_dst_id = result.node_id;
                    copy_subtree(src, child_src_id, dst, child_dst_id);
                }
            }
        }
        NodeValue::Tuple(tuple) => {
            for (idx, &child_src_id) in tuple.iter().enumerate() {
                if let Ok(result) = dst.add_tuple_element(idx as u8, dst_id) {
                    let child_dst_id = result.node_id;
                    copy_subtree(src, child_src_id, dst, child_dst_id);
                }
            }
        }
        NodeValue::Map(map) => {
            for (key, &child_src_id) in map.iter() {
                if let Ok(result) = dst.add_map_child(key.clone(), dst_id) {
                    let child_dst_id = result.node_id;
                    copy_subtree(src, child_src_id, dst, child_dst_id);
                }
            }
        }
        _ => {}
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArraySchema, Bound, RecordFieldSchema};
    use eure_document::text::Text;

    fn create_simple_schema(content: SchemaNodeContent) -> (SchemaDocument, SchemaNodeId) {
        let mut schema = SchemaDocument {
            nodes: Vec::new(),
            root: SchemaNodeId(0),
            types: std::collections::HashMap::new(),
        };
        let id = schema.create_node(content);
        schema.root = id;
        (schema, id)
    }

    fn create_doc_with_primitive(value: PrimitiveValue) -> EureDocument {
        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Primitive(value);
        doc
    }

    #[test]
    fn test_validate_text_basic() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Text(TextSchema::default()));
        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string())));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_text_pattern() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Text(TextSchema {
            pattern: Some(regex::Regex::new("^[a-z]+$").unwrap()),
            ..Default::default()
        }));

        let doc =
            create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext("hello".to_string())));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Text(Text::plaintext(
            "Hello123".to_string(),
        )));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_integer() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Integer(IntegerSchema {
            min: Bound::Inclusive(BigInt::from(0)),
            max: Bound::Inclusive(BigInt::from(100)),
            multiple_of: None,
        }));

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(50)));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(150)));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_boolean() {
        let (schema, _) = create_simple_schema(SchemaNodeContent::Boolean);

        let doc = create_doc_with_primitive(PrimitiveValue::Bool(true));
        let result = validate(&doc, &schema);
        assert!(result.is_valid);

        let doc = create_doc_with_primitive(PrimitiveValue::Integer(BigInt::from(1)));
        let result = validate(&doc, &schema);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_validate_array() {
        let (mut schema, _) = create_simple_schema(SchemaNodeContent::Any);
        let item_schema_id =
            schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        schema.node_mut(schema.root).content = SchemaNodeContent::Array(ArraySchema {
            item: item_schema_id,
            min_length: Some(1),
            max_length: Some(3),
            unique: false,
            contains: None,
            binding_style: None,
        });

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Array(Default::default());
        let child1 = doc.add_array_element(None, root_id).unwrap().node_id;
        doc.node_mut(child1).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(1)));
        let child2 = doc.add_array_element(None, root_id).unwrap().node_id;
        doc.node_mut(child2).content =
            NodeValue::Primitive(PrimitiveValue::Integer(BigInt::from(2)));

        let result = validate(&doc, &schema);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_record() {
        let (mut schema, _) = create_simple_schema(SchemaNodeContent::Any);
        let name_schema_id = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let age_schema_id =
            schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));

        let mut properties = std::collections::HashMap::new();
        properties.insert(
            "name".to_string(),
            RecordFieldSchema {
                schema: name_schema_id,
                optional: false,
                binding_style: None,
            },
        );
        properties.insert(
            "age".to_string(),
            RecordFieldSchema {
                schema: age_schema_id,
                optional: true,
                binding_style: None,
            },
        );

        schema.node_mut(schema.root).content = SchemaNodeContent::Record(RecordSchema {
            properties,
            unknown_fields: UnknownFieldsPolicy::Deny,
        });

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        let name_id = doc
            .add_map_child(ObjectKey::String("name".to_string()), root_id)
            .unwrap()
            .node_id;
        doc.node_mut(name_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("Alice".to_string())));

        let result = validate(&doc, &schema);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_hole() {
        let (schema, _) =
            create_simple_schema(SchemaNodeContent::Integer(IntegerSchema::default()));

        let mut doc = EureDocument::new();
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content = NodeValue::Hole(None);

        let result = validate(&doc, &schema);
        assert!(result.is_valid);
        assert!(!result.is_complete);
    }
}
