//! Conversion from SchemaDocument to SourceDocument.
//!
//! This module provides functionality to convert Eure Schema documents back to
//! Eure source format, enabling round-trip schema editing and serialization.

use eure_document::Text;
use eure_document::document::node::NodeValue;
use eure_document::document::{EureDocument, NodeId};
use eure_document::identifier::Identifier;
use eure_document::source::{Layout, LayoutItem, SectionBody, SourceDocument, SourcePathSegment};
use eure_document::value::{ObjectKey, PrimitiveValue};
use thiserror::Error;

use crate::{
    ArraySchema, Bound, Description, FloatPrecision, FloatSchema, IntegerSchema, MapSchema,
    RecordSchema, SchemaDocument, SchemaMetadata, SchemaNode, SchemaNodeContent, SchemaNodeId,
    TextSchema, TupleSchema, TypeReference, UnionSchema, UnknownFieldsPolicy,
};

/// Errors that can occur during SchemaDocument to SourceDocument conversion.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ToSourceError {
    /// Invalid schema node ID reference.
    #[error("Invalid schema node ID: {0}")]
    InvalidNodeId(usize),

    /// Circular reference detected during conversion.
    #[error("Circular reference detected: {0}")]
    CircularReference(String),
}

/// Convert a SchemaDocument to a SourceDocument.
///
/// This function generates a well-formatted Eure source representation of the schema.
pub fn schema_to_source_document(schema: &SchemaDocument) -> Result<SourceDocument, ToSourceError> {
    let converter = SchemaToSourceConverter::new(schema);
    converter.convert()
}

/// Converter state for SchemaDocument to SourceDocument conversion.
struct SchemaToSourceConverter<'a> {
    schema: &'a SchemaDocument,
    document: EureDocument,
    layout: Layout,
    /// Track visited nodes for cycle detection.
    visiting: Vec<SchemaNodeId>,
}

impl<'a> SchemaToSourceConverter<'a> {
    fn new(schema: &'a SchemaDocument) -> Self {
        Self {
            schema,
            document: EureDocument::new_empty(),
            layout: Layout::new(),
            visiting: Vec::new(),
        }
    }

    fn convert(mut self) -> Result<SourceDocument, ToSourceError> {
        // 1. Convert named types into $types bindings
        self.convert_named_types()?;

        // 2. Convert root schema (if it's not just Any)
        self.convert_root()?;

        Ok(SourceDocument::new(self.document, self.layout))
    }

    /// Get a node from the schema document.
    fn get_node(&self, id: SchemaNodeId) -> Result<&SchemaNode, ToSourceError> {
        self.schema
            .nodes
            .get(id.0)
            .ok_or(ToSourceError::InvalidNodeId(id.0))
    }

    /// Mark a node as being visited (for cycle detection).
    fn push_visiting(&mut self, id: SchemaNodeId) -> Result<(), ToSourceError> {
        if self.visiting.contains(&id) {
            return Err(ToSourceError::CircularReference(format!(
                "Node {} creates a cycle",
                id.0
            )));
        }
        self.visiting.push(id);
        Ok(())
    }

    /// Unmark a node as being visited.
    fn pop_visiting(&mut self) {
        self.visiting.pop();
    }

    // =========================================================================
    // Named Types Conversion
    // =========================================================================

    fn convert_named_types(&mut self) -> Result<(), ToSourceError> {
        let types: Vec<_> = self
            .schema
            .types
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        for (name, node_id) in types {
            let path = vec![
                SourcePathSegment::extension(Identifier::new_unchecked("types")),
                SourcePathSegment::ident(name),
            ];

            // Check if this should be a section (record/union) or a simple binding
            if self.should_use_section(node_id)? {
                // Section with nested bindings
                let section_items = self.convert_schema_to_section_items(node_id)?;
                self.layout.push(LayoutItem::Section {
                    path,
                    trailing_comment: None,
                    body: SectionBody::Items(section_items),
                });
            } else {
                // Simple binding: $types.name = `type`
                let value_node = self.convert_schema_node(node_id)?;
                self.layout.push(LayoutItem::Binding {
                    path,
                    node: value_node,
                    trailing_comment: None,
                });
            }
        }
        Ok(())
    }

    fn convert_root(&mut self) -> Result<(), ToSourceError> {
        let root_node = self.get_node(self.schema.root)?.clone();

        // Skip if root is just Any (default)
        if matches!(root_node.content, SchemaNodeContent::Any)
            && root_node.metadata == SchemaMetadata::default()
            && root_node.ext_types.is_empty()
        {
            return Ok(());
        }

        // For records at root, convert fields as top-level bindings
        if let SchemaNodeContent::Record(ref record) = root_node.content {
            let record = record.clone();
            self.convert_record_as_bindings(&record, vec![])?;
        } else {
            // Other root types get a root binding
            let value_node = self.convert_schema_node(self.schema.root)?;
            self.layout.push(LayoutItem::Binding {
                path: vec![],
                node: value_node,
                trailing_comment: None,
            });
        }

        Ok(())
    }

    /// Check if a schema node should use section syntax (for records/unions with fields).
    fn should_use_section(&self, node_id: SchemaNodeId) -> Result<bool, ToSourceError> {
        let node = self.get_node(node_id)?;
        match &node.content {
            SchemaNodeContent::Record(r) => Ok(!r.properties.is_empty()),
            SchemaNodeContent::Union(u) => Ok(!u.variants.is_empty()),
            _ => Ok(false),
        }
    }

    /// Convert a schema node to section items (for records/unions).
    fn convert_schema_to_section_items(
        &mut self,
        node_id: SchemaNodeId,
    ) -> Result<Vec<LayoutItem>, ToSourceError> {
        let node = self.get_node(node_id)?.clone();
        let mut items = Vec::new();

        // Add metadata first
        self.add_metadata_items(&node.metadata, &mut items)?;

        match &node.content {
            SchemaNodeContent::Record(record) => {
                let record = record.clone();
                self.add_record_items(&record, &mut items)?;
            }
            SchemaNodeContent::Union(union) => {
                let union = union.clone();
                self.add_union_items(&union, &mut items)?;
            }
            _ => {
                // For other types in section context, add as binding
                let value_node = self.convert_schema_node(node_id)?;
                items.push(LayoutItem::Binding {
                    path: vec![],
                    node: value_node,
                    trailing_comment: None,
                });
            }
        }

        Ok(items)
    }

    // =========================================================================
    // Schema Node Conversion
    // =========================================================================

    /// Convert a schema node to a document node ID.
    fn convert_schema_node(&mut self, node_id: SchemaNodeId) -> Result<NodeId, ToSourceError> {
        self.push_visiting(node_id)?;

        let node = self.get_node(node_id)?.clone();
        let result = self.convert_schema_content(&node.content, &node.metadata)?;

        self.pop_visiting();
        Ok(result)
    }

    /// Convert schema content to a document node.
    fn convert_schema_content(
        &mut self,
        content: &SchemaNodeContent,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        // Check if we can use shorthand (no constraints, no metadata)
        if metadata == &SchemaMetadata::default()
            && let Some(shorthand) = self.try_primitive_shorthand(content)
        {
            return self.create_inline_code(&shorthand);
        }

        // Otherwise use expanded form
        match content {
            SchemaNodeContent::Any => self.create_inline_code("any"),
            SchemaNodeContent::Text(t) => self.convert_text_schema(t, metadata),
            SchemaNodeContent::Integer(i) => self.convert_integer_schema(i, metadata),
            SchemaNodeContent::Float(f) => self.convert_float_schema(f, metadata),
            SchemaNodeContent::Boolean => {
                if metadata == &SchemaMetadata::default() {
                    self.create_inline_code("boolean")
                } else {
                    self.convert_constrained_primitive("boolean", metadata)
                }
            }
            SchemaNodeContent::Null => {
                if metadata == &SchemaMetadata::default() {
                    self.create_inline_code("null")
                } else {
                    self.convert_constrained_primitive("null", metadata)
                }
            }
            SchemaNodeContent::Literal(doc) => self.convert_literal(doc),
            SchemaNodeContent::Array(a) => {
                let a = a.clone();
                self.convert_array_schema(&a, metadata)
            }
            SchemaNodeContent::Map(m) => {
                let m = m.clone();
                self.convert_map_schema(&m, metadata)
            }
            SchemaNodeContent::Record(r) => {
                let r = r.clone();
                self.convert_record_schema(&r, metadata)
            }
            SchemaNodeContent::Tuple(t) => {
                let t = t.clone();
                self.convert_tuple_schema(&t, metadata)
            }
            SchemaNodeContent::Union(u) => {
                let u = u.clone();
                self.convert_union_schema(&u, metadata)
            }
            SchemaNodeContent::Reference(r) => self.convert_reference(r),
        }
    }

    /// Try to get a shorthand representation for a primitive type.
    fn try_primitive_shorthand(&self, content: &SchemaNodeContent) -> Option<String> {
        match content {
            SchemaNodeContent::Any => Some("any".to_string()),
            SchemaNodeContent::Text(t) if t.is_default_except_language() => match &t.language {
                Some(lang) => Some(format!("text.{}", lang)),
                None => Some("text".to_string()),
            },
            SchemaNodeContent::Integer(i) if i.is_default() => Some("integer".to_string()),
            SchemaNodeContent::Float(f) if f.is_default() => Some("float".to_string()),
            SchemaNodeContent::Boolean => Some("boolean".to_string()),
            SchemaNodeContent::Null => Some("null".to_string()),
            _ => None,
        }
    }

    // =========================================================================
    // Primitive Type Conversion
    // =========================================================================

    fn convert_text_schema(
        &mut self,
        text: &TextSchema,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        // Check if we can use shorthand
        if text.is_default_except_language() && metadata == &SchemaMetadata::default() {
            let shorthand = match &text.language {
                Some(lang) => format!("text.{}", lang),
                None => "text".to_string(),
            };
            return self.create_inline_code(&shorthand);
        }

        // Expanded form: { type = `text`, ... }
        let mut fields = Vec::new();

        // type field
        let type_node = self.create_inline_code("text")?;
        fields.push(("type".to_string(), type_node));

        if let Some(lang) = &text.language {
            let lang_node = self.create_string_value(lang)?;
            fields.push(("language".to_string(), lang_node));
        }

        if let Some(min) = text.min_length {
            let min_node = self.create_integer_value(min as i64)?;
            fields.push(("min-length".to_string(), min_node));
        }

        if let Some(max) = text.max_length {
            let max_node = self.create_integer_value(max as i64)?;
            fields.push(("max-length".to_string(), max_node));
        }

        if let Some(pattern) = &text.pattern {
            let pattern_node = self.create_string_value(pattern.as_str())?;
            fields.push(("pattern".to_string(), pattern_node));
        }

        self.create_map_with_metadata(fields, metadata)
    }

    fn convert_integer_schema(
        &mut self,
        int: &IntegerSchema,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        if int.is_default() && metadata == &SchemaMetadata::default() {
            return self.create_inline_code("integer");
        }

        let mut fields = Vec::new();

        let type_node = self.create_inline_code("integer")?;
        fields.push(("type".to_string(), type_node));

        // Convert range if present
        if let Some(range_str) = self.bounds_to_range_string(&int.min, &int.max) {
            let range_node = self.create_string_value(&range_str)?;
            fields.push(("range".to_string(), range_node));
        }

        if let Some(mult) = &int.multiple_of {
            let mult_node = self.create_integer_value(mult.try_into().unwrap_or(0))?;
            fields.push(("multiple-of".to_string(), mult_node));
        }

        self.create_map_with_metadata(fields, metadata)
    }

    fn convert_float_schema(
        &mut self,
        float: &FloatSchema,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        if float.is_default() && metadata == &SchemaMetadata::default() {
            return self.create_inline_code("float");
        }

        let mut fields = Vec::new();

        let type_node = self.create_inline_code("float")?;
        fields.push(("type".to_string(), type_node));

        // Convert range if present
        if let Some(range_str) = self.float_bounds_to_range_string(&float.min, &float.max) {
            let range_node = self.create_string_value(&range_str)?;
            fields.push(("range".to_string(), range_node));
        }

        if let Some(mult) = &float.multiple_of {
            let mult_node = self.create_float_value(*mult)?;
            fields.push(("multiple-of".to_string(), mult_node));
        }

        if float.precision != FloatPrecision::default() {
            let precision_str = match float.precision {
                FloatPrecision::F32 => "f32",
                FloatPrecision::F64 => "f64",
            };
            let precision_node = self.create_string_value(precision_str)?;
            fields.push(("precision".to_string(), precision_node));
        }

        self.create_map_with_metadata(fields, metadata)
    }

    fn convert_constrained_primitive(
        &mut self,
        type_name: &str,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        let mut fields = Vec::new();
        let type_node = self.create_inline_code(type_name)?;
        fields.push(("type".to_string(), type_node));
        self.create_map_with_metadata(fields, metadata)
    }

    // =========================================================================
    // Compound Type Conversion
    // =========================================================================

    fn convert_array_schema(
        &mut self,
        array: &ArraySchema,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        // Check if we can use shorthand: [`item`]
        if array.min_length.is_none()
            && array.max_length.is_none()
            && !array.unique
            && array.contains.is_none()
            && array.binding_style.is_none()
            && metadata == &SchemaMetadata::default()
        {
            let item_node = self.convert_schema_node(array.item)?;
            return self.create_array_value(vec![item_node]);
        }

        // Expanded form
        let mut fields = Vec::new();

        let type_node = self.create_inline_code("array")?;
        fields.push(("type".to_string(), type_node));

        let item_node = self.convert_schema_node(array.item)?;
        fields.push(("item".to_string(), item_node));

        if let Some(min) = array.min_length {
            let min_node = self.create_integer_value(min as i64)?;
            fields.push(("min-length".to_string(), min_node));
        }

        if let Some(max) = array.max_length {
            let max_node = self.create_integer_value(max as i64)?;
            fields.push(("max-length".to_string(), max_node));
        }

        if array.unique {
            let unique_node = self.create_boolean_value(true)?;
            fields.push(("unique".to_string(), unique_node));
        }

        if let Some(contains_id) = array.contains {
            let contains_node = self.convert_schema_node(contains_id)?;
            fields.push(("contains".to_string(), contains_node));
        }

        self.create_map_with_metadata(fields, metadata)
    }

    fn convert_map_schema(
        &mut self,
        map: &MapSchema,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        let mut fields = Vec::new();

        let type_node = self.create_inline_code("map")?;
        fields.push(("type".to_string(), type_node));

        let key_node = self.convert_schema_node(map.key)?;
        fields.push(("key".to_string(), key_node));

        let value_node = self.convert_schema_node(map.value)?;
        fields.push(("value".to_string(), value_node));

        if let Some(min) = map.min_size {
            let min_node = self.create_integer_value(min as i64)?;
            fields.push(("min-size".to_string(), min_node));
        }

        if let Some(max) = map.max_size {
            let max_node = self.create_integer_value(max as i64)?;
            fields.push(("max-size".to_string(), max_node));
        }

        self.create_map_with_metadata(fields, metadata)
    }

    fn convert_record_schema(
        &mut self,
        record: &RecordSchema,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        // Records are represented as maps with field bindings
        let mut fields = Vec::new();

        for (field_name, field_schema) in &record.properties {
            let mut field_node = self.convert_schema_node(field_schema.schema)?;

            // If optional, we need to wrap with $optional extension
            if field_schema.optional {
                field_node = self.add_optional_extension(field_node)?;
            }

            fields.push((field_name.clone(), field_node));
        }

        // Add unknown-fields policy if not default
        match &record.unknown_fields {
            UnknownFieldsPolicy::Deny => {} // Default, don't emit
            UnknownFieldsPolicy::Allow => {
                let allow_node = self.create_string_value("allow")?;
                // This should be an extension: $unknown-fields = "allow"
                // For now, add as regular field
                fields.push(("$unknown-fields".to_string(), allow_node));
            }
            UnknownFieldsPolicy::Schema(schema_id) => {
                let schema_node = self.convert_schema_node(*schema_id)?;
                fields.push(("$unknown-fields".to_string(), schema_node));
            }
        }

        self.create_map_with_metadata(fields, metadata)
    }

    fn convert_tuple_schema(
        &mut self,
        tuple: &TupleSchema,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        // Tuples use tuple syntax: (`text`, `integer`)
        let elements: Vec<NodeId> = tuple
            .elements
            .iter()
            .map(|&id| self.convert_schema_node(id))
            .collect::<Result<_, _>>()?;

        if metadata == &SchemaMetadata::default() {
            self.create_tuple_value(elements)
        } else {
            // With metadata, use expanded form
            let mut fields = Vec::new();

            let type_node = self.create_inline_code("tuple")?;
            fields.push(("type".to_string(), type_node));

            let elements_node = self.create_array_value(elements)?;
            fields.push(("elements".to_string(), elements_node));

            self.create_map_with_metadata(fields, metadata)
        }
    }

    fn convert_union_schema(
        &mut self,
        union: &UnionSchema,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        // Union: { $variant = "union", variant1 = schema1, ... }
        let mut fields = Vec::new();

        for (variant_name, &variant_id) in &union.variants {
            let variant_node = self.convert_schema_node(variant_id)?;
            fields.push((variant_name.clone(), variant_node));
        }

        // Create map and add $variant = "union" extension
        let map_node = self.create_map_with_metadata(fields, metadata)?;
        self.add_variant_extension(map_node, "union")
    }

    // =========================================================================
    // Reference and Literal Conversion
    // =========================================================================

    fn convert_reference(&mut self, reference: &TypeReference) -> Result<NodeId, ToSourceError> {
        let ref_str = match &reference.namespace {
            Some(ns) => format!("$types.{}.{}", ns, reference.name),
            None => format!("$types.{}", reference.name),
        };
        self.create_inline_code(&ref_str)
    }

    fn convert_literal(&mut self, doc: &EureDocument) -> Result<NodeId, ToSourceError> {
        // Clone the literal document value into our document
        let value_node = self.copy_document_value(doc, doc.get_root_id())?;
        // Add $variant = "literal" extension
        self.add_variant_extension(value_node, "literal")
    }

    // =========================================================================
    // Metadata Conversion
    // =========================================================================

    fn add_metadata_items(
        &mut self,
        metadata: &SchemaMetadata,
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // $description: text
        if let Some(desc) = &metadata.description {
            let desc_text = match desc {
                Description::String(s) => s.clone(),
                Description::Markdown(s) => s.clone(),
            };
            let desc_node = self.create_string_value(&desc_text)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                    "description",
                ))],
                node: desc_node,
                trailing_comment: None,
            });
        }

        // $deprecated
        if metadata.deprecated {
            let deprecated_node = self.create_boolean_value(true)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                    "deprecated",
                ))],
                node: deprecated_node,
                trailing_comment: None,
            });
        }

        // $default = value
        if let Some(default_doc) = &metadata.default {
            let default_node = self.copy_document_value(default_doc, default_doc.get_root_id())?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                    "default",
                ))],
                node: default_node,
                trailing_comment: None,
            });
        }

        // $examples = [...]
        if let Some(examples) = &metadata.examples {
            let example_nodes: Vec<NodeId> = examples
                .iter()
                .map(|doc| self.copy_document_value(doc, doc.get_root_id()))
                .collect::<Result<_, _>>()?;
            let examples_node = self.create_array_value(example_nodes)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                    "examples",
                ))],
                node: examples_node,
                trailing_comment: None,
            });
        }

        Ok(())
    }

    fn add_record_items(
        &mut self,
        record: &RecordSchema,
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        for (field_name, field_schema) in &record.properties {
            let mut field_node = self.convert_schema_node(field_schema.schema)?;

            if field_schema.optional {
                field_node = self.add_optional_extension(field_node)?;
            }

            let ident: Identifier = field_name
                .parse()
                .unwrap_or_else(|_| Identifier::new_unchecked("invalid"));
            let path = vec![SourcePathSegment::ident(ident)];
            items.push(LayoutItem::Binding {
                path,
                node: field_node,
                trailing_comment: None,
            });
        }

        // Add unknown-fields policy if not default
        match &record.unknown_fields {
            UnknownFieldsPolicy::Deny => {}
            UnknownFieldsPolicy::Allow => {
                let allow_node = self.create_string_value("allow")?;
                items.push(LayoutItem::Binding {
                    path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                        "unknown-fields",
                    ))],
                    node: allow_node,
                    trailing_comment: None,
                });
            }
            UnknownFieldsPolicy::Schema(schema_id) => {
                let schema_node = self.convert_schema_node(*schema_id)?;
                items.push(LayoutItem::Binding {
                    path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                        "unknown-fields",
                    ))],
                    node: schema_node,
                    trailing_comment: None,
                });
            }
        }

        Ok(())
    }

    fn add_union_items(
        &mut self,
        union: &UnionSchema,
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // Add $variant = "union"
        let union_node = self.create_string_value("union")?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                "variant",
            ))],
            node: union_node,
            trailing_comment: None,
        });

        // Add variants
        for (variant_name, &variant_id) in &union.variants {
            let variant_node = self.convert_schema_node(variant_id)?;
            let ident: Identifier = variant_name
                .parse()
                .unwrap_or_else(|_| Identifier::new_unchecked("invalid"));
            let path = vec![SourcePathSegment::ident(ident)];
            items.push(LayoutItem::Binding {
                path,
                node: variant_node,
                trailing_comment: None,
            });
        }

        Ok(())
    }

    fn convert_record_as_bindings(
        &mut self,
        record: &RecordSchema,
        base_path: Vec<SourcePathSegment>,
    ) -> Result<(), ToSourceError> {
        for (field_name, field_schema) in &record.properties {
            let mut field_node = self.convert_schema_node(field_schema.schema)?;

            if field_schema.optional {
                field_node = self.add_optional_extension(field_node)?;
            }

            let mut path = base_path.clone();
            let ident: Identifier = field_name
                .parse()
                .unwrap_or_else(|_| Identifier::new_unchecked("invalid"));
            path.push(SourcePathSegment::ident(ident));

            self.layout.push(LayoutItem::Binding {
                path,
                node: field_node,
                trailing_comment: None,
            });
        }

        Ok(())
    }

    // =========================================================================
    // Helper Methods for Creating Document Nodes
    // =========================================================================

    fn create_inline_code(&mut self, code: &str) -> Result<NodeId, ToSourceError> {
        let text = Text::inline_implicit(code.to_string());
        let node_id = self
            .document
            .create_node(NodeValue::Primitive(PrimitiveValue::Text(text)));
        Ok(node_id)
    }

    fn create_string_value(&mut self, s: &str) -> Result<NodeId, ToSourceError> {
        let text = Text::plaintext(s.to_string());
        let node_id = self
            .document
            .create_node(NodeValue::Primitive(PrimitiveValue::Text(text)));
        Ok(node_id)
    }

    fn create_integer_value(&mut self, n: i64) -> Result<NodeId, ToSourceError> {
        let node_id = self
            .document
            .create_node(NodeValue::Primitive(PrimitiveValue::Integer(n.into())));
        Ok(node_id)
    }

    fn create_float_value(&mut self, n: f64) -> Result<NodeId, ToSourceError> {
        let node_id = self
            .document
            .create_node(NodeValue::Primitive(PrimitiveValue::F64(n)));
        Ok(node_id)
    }

    fn create_boolean_value(&mut self, b: bool) -> Result<NodeId, ToSourceError> {
        let node_id = self
            .document
            .create_node(NodeValue::Primitive(PrimitiveValue::Bool(b)));
        Ok(node_id)
    }

    fn create_array_value(&mut self, elements: Vec<NodeId>) -> Result<NodeId, ToSourceError> {
        let array_id = self
            .document
            .create_node(NodeValue::Array(Default::default()));
        for element in elements {
            // Use the add_array_element API
            let _ = self.document.add_array_element(None, array_id);
            // Set the content of the last added element
            if let Some(arr) = self.document.node(array_id).as_array()
                && let Some(last_idx) = arr.len().checked_sub(1)
                && let Some(last_id) = arr.get(last_idx)
            {
                // Copy content from element to last_id
                let element_content = self.document.node(element).content.clone();
                let element_extensions = self.document.node(element).extensions.clone();
                self.document.node_mut(last_id).content = element_content;
                self.document.node_mut(last_id).extensions = element_extensions;
            }
        }
        Ok(array_id)
    }

    fn create_tuple_value(&mut self, elements: Vec<NodeId>) -> Result<NodeId, ToSourceError> {
        let tuple_id = self
            .document
            .create_node(NodeValue::Tuple(Default::default()));
        for (index, element) in elements.into_iter().enumerate() {
            let _ = self.document.add_tuple_element(index as u8, tuple_id);
            // Set the content of the added element
            if let Some(tup) = self.document.node(tuple_id).as_tuple()
                && let Some(elem_id) = tup.get(index)
            {
                let element_content = self.document.node(element).content.clone();
                let element_extensions = self.document.node(element).extensions.clone();
                self.document.node_mut(elem_id).content = element_content;
                self.document.node_mut(elem_id).extensions = element_extensions;
            }
        }
        Ok(tuple_id)
    }

    fn create_map_with_metadata(
        &mut self,
        fields: Vec<(String, NodeId)>,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        let map_id = self
            .document
            .create_node(NodeValue::Map(Default::default()));

        for (key, value_id) in fields {
            let object_key = ObjectKey::String(key);
            let _ = self.document.add_map_child(object_key.clone(), map_id);
            // Set the content of the added child
            if let Some(map) = self.document.node(map_id).as_map()
                && let Some(&child_id) = map.get(&object_key)
            {
                let value_content = self.document.node(value_id).content.clone();
                let value_extensions = self.document.node(value_id).extensions.clone();
                self.document.node_mut(child_id).content = value_content;
                self.document.node_mut(child_id).extensions = value_extensions;
            }
        }

        // Add metadata as extensions if present
        self.add_metadata_extensions(map_id, metadata)?;

        Ok(map_id)
    }

    fn add_metadata_extensions(
        &mut self,
        node_id: NodeId,
        metadata: &SchemaMetadata,
    ) -> Result<(), ToSourceError> {
        if let Some(desc) = &metadata.description {
            let desc_text = match desc {
                Description::String(s) => s.clone(),
                Description::Markdown(s) => s.clone(),
            };
            let desc_node = self.create_string_value(&desc_text)?;
            let node = self.document.node_mut(node_id);
            node.extensions
                .insert(Identifier::new_unchecked("description"), desc_node);
        }

        if metadata.deprecated {
            let deprecated_node = self.create_boolean_value(true)?;
            let node = self.document.node_mut(node_id);
            node.extensions
                .insert(Identifier::new_unchecked("deprecated"), deprecated_node);
        }

        if let Some(default_doc) = &metadata.default {
            let default_node = self.copy_document_value(default_doc, default_doc.get_root_id())?;
            let node = self.document.node_mut(node_id);
            node.extensions
                .insert(Identifier::new_unchecked("default"), default_node);
        }

        if let Some(examples) = &metadata.examples {
            let example_nodes: Vec<NodeId> = examples
                .iter()
                .map(|doc| self.copy_document_value(doc, doc.get_root_id()))
                .collect::<Result<_, _>>()?;
            let examples_node = self.create_array_value(example_nodes)?;
            let node = self.document.node_mut(node_id);
            node.extensions
                .insert(Identifier::new_unchecked("examples"), examples_node);
        }

        Ok(())
    }

    fn add_optional_extension(&mut self, node_id: NodeId) -> Result<NodeId, ToSourceError> {
        let optional_node = self.create_boolean_value(true)?;
        let node = self.document.node_mut(node_id);
        node.extensions
            .insert(Identifier::new_unchecked("optional"), optional_node);
        Ok(node_id)
    }

    fn add_variant_extension(
        &mut self,
        node_id: NodeId,
        variant: &str,
    ) -> Result<NodeId, ToSourceError> {
        let variant_node = self.create_string_value(variant)?;
        let node = self.document.node_mut(node_id);
        node.extensions
            .insert(Identifier::new_unchecked("variant"), variant_node);
        Ok(node_id)
    }

    fn copy_document_value(
        &mut self,
        source_doc: &EureDocument,
        source_id: NodeId,
    ) -> Result<NodeId, ToSourceError> {
        let source_node = source_doc.node(source_id);

        let new_id = match &source_node.content {
            NodeValue::Primitive(p) => self.document.create_node(NodeValue::Primitive(p.clone())),
            NodeValue::Array(arr) => {
                let array_id = self
                    .document
                    .create_node(NodeValue::Array(Default::default()));
                for &child_id in arr.iter() {
                    let child_copy = self.copy_document_value(source_doc, child_id)?;
                    let _ = self.document.add_array_element(None, array_id);
                    if let Some(arr) = self.document.node(array_id).as_array()
                        && let Some(last_idx) = arr.len().checked_sub(1)
                        && let Some(last_id) = arr.get(last_idx)
                    {
                        let child_content = self.document.node(child_copy).content.clone();
                        let child_extensions = self.document.node(child_copy).extensions.clone();
                        self.document.node_mut(last_id).content = child_content;
                        self.document.node_mut(last_id).extensions = child_extensions;
                    }
                }
                array_id
            }
            NodeValue::Tuple(tup) => {
                let tuple_id = self
                    .document
                    .create_node(NodeValue::Tuple(Default::default()));
                for (idx, &child_id) in tup.iter().enumerate() {
                    let child_copy = self.copy_document_value(source_doc, child_id)?;
                    let _ = self.document.add_tuple_element(idx as u8, tuple_id);
                    if let Some(tup) = self.document.node(tuple_id).as_tuple()
                        && let Some(elem_id) = tup.get(idx)
                    {
                        let child_content = self.document.node(child_copy).content.clone();
                        let child_extensions = self.document.node(child_copy).extensions.clone();
                        self.document.node_mut(elem_id).content = child_content;
                        self.document.node_mut(elem_id).extensions = child_extensions;
                    }
                }
                tuple_id
            }
            NodeValue::Map(map) => {
                let map_id = self
                    .document
                    .create_node(NodeValue::Map(Default::default()));
                for (key, &child_id) in map.iter() {
                    let child_copy = self.copy_document_value(source_doc, child_id)?;
                    let _ = self.document.add_map_child(key.clone(), map_id);
                    if let Some(map) = self.document.node(map_id).as_map()
                        && let Some(&new_child_id) = map.get(key)
                    {
                        let child_content = self.document.node(child_copy).content.clone();
                        let child_extensions = self.document.node(child_copy).extensions.clone();
                        self.document.node_mut(new_child_id).content = child_content;
                        self.document.node_mut(new_child_id).extensions = child_extensions;
                    }
                }
                map_id
            }
            NodeValue::Hole(h) => self.document.create_node(NodeValue::Hole(h.clone())),
        };

        // Copy extensions
        for (ext_key, &ext_value_id) in &source_node.extensions {
            let ext_copy = self.copy_document_value(source_doc, ext_value_id)?;
            let node = self.document.node_mut(new_id);
            node.extensions.insert(ext_key.clone(), ext_copy);
        }

        Ok(new_id)
    }

    // =========================================================================
    // Range String Conversion
    // =========================================================================

    fn bounds_to_range_string<T: std::fmt::Display>(
        &self,
        min: &Bound<T>,
        max: &Bound<T>,
    ) -> Option<String> {
        match (min, max) {
            (Bound::Unbounded, Bound::Unbounded) => None,
            (Bound::Inclusive(min), Bound::Inclusive(max)) => Some(format!("{}..={}", min, max)),
            (Bound::Inclusive(min), Bound::Exclusive(max)) => Some(format!("{}..{}", min, max)),
            (Bound::Exclusive(min), Bound::Inclusive(max)) => Some(format!("{}<..={}", min, max)),
            (Bound::Exclusive(min), Bound::Exclusive(max)) => Some(format!("{}<..{}", min, max)),
            (Bound::Inclusive(min), Bound::Unbounded) => Some(format!("{}..", min)),
            (Bound::Exclusive(min), Bound::Unbounded) => Some(format!("{}<..", min)),
            (Bound::Unbounded, Bound::Inclusive(max)) => Some(format!("..={}", max)),
            (Bound::Unbounded, Bound::Exclusive(max)) => Some(format!("..{}", max)),
        }
    }

    fn float_bounds_to_range_string(&self, min: &Bound<f64>, max: &Bound<f64>) -> Option<String> {
        self.bounds_to_range_string(min, max)
    }
}

// ============================================================================
// Helper trait implementations
// ============================================================================

impl TextSchema {
    /// Check if the TextSchema has default values except for language.
    fn is_default_except_language(&self) -> bool {
        self.min_length.is_none()
            && self.max_length.is_none()
            && self.pattern.is_none()
            && self.unknown_fields.is_empty()
    }
}

impl IntegerSchema {
    /// Check if the IntegerSchema has all default values.
    fn is_default(&self) -> bool {
        matches!(self.min, Bound::Unbounded)
            && matches!(self.max, Bound::Unbounded)
            && self.multiple_of.is_none()
    }
}

impl FloatSchema {
    /// Check if the FloatSchema has all default values.
    fn is_default(&self) -> bool {
        matches!(self.min, Bound::Unbounded)
            && matches!(self.max, Bound::Unbounded)
            && self.multiple_of.is_none()
            && self.precision == FloatPrecision::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_shorthand() {
        let mut schema = SchemaDocument::new();
        schema.root = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));

        let result = schema_to_source_document(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_reference() {
        let mut schema = SchemaDocument::new();

        // Create a named type
        let text_id = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));
        schema.register_type(Identifier::new_unchecked("username"), text_id);

        // Create a reference to it
        schema.root = schema.create_node(SchemaNodeContent::Reference(TypeReference {
            namespace: None,
            name: Identifier::new_unchecked("username"),
        }));

        let result = schema_to_source_document(&schema);
        assert!(result.is_ok());
    }
}
