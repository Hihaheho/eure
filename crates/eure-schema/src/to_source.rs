//! Conversion from SchemaDocument to SourceDocument.
//!
//! This module provides functionality to convert Eure Schema documents back to
//! Eure source format, enabling round-trip schema editing and serialization.

use eure_document::document::constructor::DocumentConstructor;
use eure_document::document::node::NodeValue;
use eure_document::document::{EureDocument, NodeId};
use eure_document::identifier::Identifier;
use eure_document::path::PathSegment;
use eure_document::source::{Layout, LayoutItem, SectionBody, SourceDocument, SourcePathSegment};
use eure_document::text::Text;
use eure_document::value::{ObjectKey, PrimitiveValue};
use eure_document::data_model::VariantRepr;
use thiserror::Error;

use crate::{
    ArraySchema, Bound, Description, FloatPrecision, FloatSchema, IntegerSchema, MapSchema,
    RangeStyle, RecordSchema, SchemaDocument, SchemaMetadata, SchemaNode, SchemaNodeContent,
    SchemaNodeId, TextSchema, TupleSchema, TypeReference, UnionSchema, UnknownFieldsPolicy,
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
    /// DocumentConstructor is used as a node factory. The root is bound to an empty array,
    /// and each new node is created by appending to this array.
    constructor: DocumentConstructor,
    layout: Layout,
    /// Track visited nodes for cycle detection.
    visiting: Vec<SchemaNodeId>,
}

impl<'a> SchemaToSourceConverter<'a> {
    fn new(schema: &'a SchemaDocument) -> Self {
        let mut constructor = DocumentConstructor::new();
        // Bind root as array to use as node pool
        constructor.bind_empty_array().unwrap();

        Self {
            schema,
            constructor,
            layout: Layout::new(),
            visiting: Vec::new(),
        }
    }

    fn convert(mut self) -> Result<SourceDocument, ToSourceError> {
        // 1. Convert root schema first (generates bindings and sections)
        self.convert_root()?;

        // 2. Convert named types (may generate sections)
        self.convert_named_types()?;

        // 3. Reorder: bindings must come before sections per eure grammar
        //    Eure: [ ValueBinding ] { Binding } { Section } ;
        let items = std::mem::take(&mut self.layout.items);
        let (bindings, sections): (Vec<_>, Vec<_>) = items
            .into_iter()
            .partition(|item| matches!(item, LayoutItem::Binding { .. }));

        for item in bindings {
            self.layout.push(item);
        }
        for item in sections {
            self.layout.push(item);
        }

        Ok(SourceDocument::new(self.constructor.finish(), self.layout))
    }

    /// Create a new node using eure! macro syntax.
    /// The closure receives a DocumentConstructor positioned at a new array element.
    fn with_new_node<F>(&mut self, f: F) -> Result<NodeId, ToSourceError>
    where
        F: FnOnce(&mut DocumentConstructor),
    {
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::ArrayIndex(None))
            .map_err(|e| ToSourceError::CircularReference(e.to_string()))?;
        f(&mut self.constructor);
        let node_id = self.constructor.current_node_id();
        self.constructor
            .end_scope(scope)
            .map_err(|e| ToSourceError::CircularReference(e.to_string()))?;
        Ok(node_id)
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
                let mut section_items = self.convert_schema_to_section_items(node_id)?;

                // Add $ext-type extensions to section_items (inside the section body)
                let schema_node = self.get_node(node_id)?.clone();
                for (ext_name, ext_schema) in schema_node.ext_types {
                    // $ext-type.<ext-name> = <schema>
                    let mut ext_path = vec![SourcePathSegment::extension(
                        Identifier::new_unchecked("ext-type"),
                    )];
                    ext_path.push(SourcePathSegment::ident(ext_name.clone()));

                    let ext_node = self.convert_schema_node(ext_schema.schema)?;
                    section_items.push(LayoutItem::Binding {
                        path: ext_path.clone(),
                        node: ext_node,
                        trailing_comment: None,
                    });

                    // $ext-type.<ext-name>.$optional = true (if optional)
                    if ext_schema.optional {
                        let optional_node = self.create_boolean_value(true)?;
                        let mut optional_path = ext_path;
                        optional_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                            "optional",
                        )));
                        section_items.push(LayoutItem::Binding {
                            path: optional_path,
                            node: optional_node,
                            trailing_comment: None,
                        });
                    }
                }

                // Use Block syntax if section items contain nested sections
                let body = if contains_nested_sections(&section_items) {
                    SectionBody::Block(section_items)
                } else {
                    SectionBody::Items(section_items)
                };
                self.layout.push(LayoutItem::Section {
                    path: path.clone(),
                    trailing_comment: None,
                    body,
                });
            } else {
                // Simple binding: $types.name = `type`
                let value_node = self.convert_schema_node(node_id)?;
                self.layout.push(LayoutItem::Binding {
                    path: path.clone(),
                    node: value_node,
                    trailing_comment: None,
                });

                // Output $ext-type extensions for this named type (as separate bindings)
                let schema_node = self.get_node(node_id)?.clone();
                for (ext_name, ext_schema) in schema_node.ext_types {
                    // $types.<name>.$ext-type.<ext-name> = <schema>
                    let mut ext_path = path.clone();
                    ext_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                        "ext-type",
                    )));
                    ext_path.push(SourcePathSegment::ident(ext_name.clone()));

                    let ext_node = self.convert_schema_node(ext_schema.schema)?;
                    self.layout.push(LayoutItem::Binding {
                        path: ext_path.clone(),
                        node: ext_node,
                        trailing_comment: None,
                    });

                    // $types.<name>.$ext-type.<ext-name>.$optional = true (if optional)
                    if ext_schema.optional {
                        let optional_node = self.create_boolean_value(true)?;
                        let mut optional_path = ext_path;
                        optional_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                            "optional",
                        )));
                        self.layout.push(LayoutItem::Binding {
                            path: optional_path,
                            node: optional_node,
                            trailing_comment: None,
                        });
                    }
                }
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
        } else if matches!(root_node.content, SchemaNodeContent::Tuple(_))
            || self.should_use_section(self.schema.root)?
            || root_node.metadata != SchemaMetadata::default()
        {
            // For tuples/maps/unions at root, use section items directly
            // This preserves $variant = "tuple", $variant = "map", etc.
            // Also use section items when there's metadata ($description, etc.)
            let section_items = self.convert_schema_to_section_items(self.schema.root)?;
            for item in section_items {
                self.layout.push(item);
            }
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

    /// Check if a schema node should use section syntax.
    /// NOTE: Metadata alone does not require section syntax - it's output as separate bindings.
    fn should_use_section(&self, node_id: SchemaNodeId) -> Result<bool, ToSourceError> {
        let node = self.get_node(node_id)?;

        match &node.content {
            SchemaNodeContent::Record(r) => Ok(!r.properties.is_empty()),
            SchemaNodeContent::Union(u) => Ok(!u.variants.is_empty()),
            // Maps always need section syntax to preserve $variant = "map"
            SchemaNodeContent::Map(_) => Ok(true),
            // Arrays need section syntax if:
            // - prefer_inline is explicitly false, OR
            // - Array has constraints
            SchemaNodeContent::Array(a) => Ok(a.prefer_inline == Some(false)
                || a.min_length.is_some()
                || a.max_length.is_some()
                || a.unique
                || a.contains.is_some()
                || a.binding_style.is_some()),
            // Tuples with binding style need section syntax
            SchemaNodeContent::Tuple(t) => Ok(t.binding_style.is_some()),
            // Constrained primitives need section syntax to preserve $variant extension
            SchemaNodeContent::Integer(i) => Ok(!matches!(i.min, Bound::Unbounded)
                || !matches!(i.max, Bound::Unbounded)
                || i.multiple_of.is_some()),
            SchemaNodeContent::Float(f) => Ok(!matches!(f.min, Bound::Unbounded)
                || !matches!(f.max, Bound::Unbounded)
                || f.multiple_of.is_some()),
            SchemaNodeContent::Text(t) => {
                Ok(t.min_length.is_some() || t.max_length.is_some() || t.pattern.is_some())
            }
            _ => Ok(false),
        }
    }

    /// Check if an item schema needs section syntax due to extensions like $variant-repr.
    /// This is used to determine if an array can use shorthand syntax.
    fn item_needs_section_syntax(&self, node_id: SchemaNodeId) -> Result<bool, ToSourceError> {
        let _node = self.get_node(node_id)?;
        // Currently all item types can express their extensions in inline form
        // (unions use $variant => "union" and $variant-repr => "..." extensions on the map)
        Ok(false)
    }

    /// Check if a schema node is a constrained primitive (text/integer/float with constraints).
    /// These should use flat bindings in records, not section syntax.
    fn is_constrained_primitive(&self, node_id: SchemaNodeId) -> Result<bool, ToSourceError> {
        let node = self.get_node(node_id)?;

        match &node.content {
            SchemaNodeContent::Integer(i) => Ok(!matches!(i.min, Bound::Unbounded)
                || !matches!(i.max, Bound::Unbounded)
                || i.multiple_of.is_some()),
            SchemaNodeContent::Float(f) => Ok(!matches!(f.min, Bound::Unbounded)
                || !matches!(f.max, Bound::Unbounded)
                || f.multiple_of.is_some()),
            SchemaNodeContent::Text(t) => {
                Ok(t.min_length.is_some() || t.max_length.is_some() || t.pattern.is_some())
            }
            _ => Ok(false),
        }
    }

    /// Output a constrained primitive as flat bindings with a path prefix.
    /// e.g., `field.$variant = "text"`, `field.min-length = 3`
    fn output_constrained_primitive_as_bindings(
        &mut self,
        node_id: SchemaNodeId,
        base_path: &[SourcePathSegment],
    ) -> Result<(), ToSourceError> {
        let node = self.get_node(node_id)?.clone();

        match &node.content {
            SchemaNodeContent::Text(text) => {
                // field.$variant = "text"
                let variant_node = self.create_string_value("text")?;
                let mut variant_path = base_path.to_vec();
                variant_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "variant",
                )));
                self.layout.push(LayoutItem::Binding {
                    path: variant_path,
                    node: variant_node,
                    trailing_comment: None,
                });

                // field.min-length = N
                if let Some(min) = text.min_length {
                    let min_node = self.create_integer_value(min as i64)?;
                    let mut min_path = base_path.to_vec();
                    min_path.push(SourcePathSegment::ident(Identifier::new_unchecked(
                        "min-length",
                    )));
                    self.layout.push(LayoutItem::Binding {
                        path: min_path,
                        node: min_node,
                        trailing_comment: None,
                    });
                }

                // field.max-length = N
                if let Some(max) = text.max_length {
                    let max_node = self.create_integer_value(max as i64)?;
                    let mut max_path = base_path.to_vec();
                    max_path.push(SourcePathSegment::ident(Identifier::new_unchecked(
                        "max-length",
                    )));
                    self.layout.push(LayoutItem::Binding {
                        path: max_path,
                        node: max_node,
                        trailing_comment: None,
                    });
                }

                // field.pattern = `...` (inline code)
                if let Some(ref pattern) = text.pattern {
                    let pattern_node = self.create_inline_code(pattern.as_str())?;
                    let mut pattern_path = base_path.to_vec();
                    pattern_path.push(SourcePathSegment::ident(Identifier::new_unchecked(
                        "pattern",
                    )));
                    self.layout.push(LayoutItem::Binding {
                        path: pattern_path,
                        node: pattern_node,
                        trailing_comment: None,
                    });
                }
            }
            SchemaNodeContent::Integer(int) => {
                // field.$variant = "integer"
                let variant_node = self.create_string_value("integer")?;
                let mut variant_path = base_path.to_vec();
                variant_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "variant",
                )));
                self.layout.push(LayoutItem::Binding {
                    path: variant_path,
                    node: variant_node,
                    trailing_comment: None,
                });

                // field.range = "..."
                if let Some(range_str) =
                    self.bounds_to_range_string(&int.min, &int.max, int.range_style)
                {
                    let range_node = self.create_string_value(&range_str)?;
                    let mut range_path = base_path.to_vec();
                    range_path.push(SourcePathSegment::ident(Identifier::new_unchecked("range")));
                    self.layout.push(LayoutItem::Binding {
                        path: range_path,
                        node: range_node,
                        trailing_comment: None,
                    });
                }

                // field.multiple-of = N
                if let Some(mult) = &int.multiple_of {
                    let mult_node =
                        self.create_integer_value(mult.clone().try_into().unwrap_or(0))?;
                    let mut mult_path = base_path.to_vec();
                    mult_path.push(SourcePathSegment::ident(Identifier::new_unchecked(
                        "multiple-of",
                    )));
                    self.layout.push(LayoutItem::Binding {
                        path: mult_path,
                        node: mult_node,
                        trailing_comment: None,
                    });
                }
            }
            SchemaNodeContent::Float(float) => {
                // field.$variant = "float"
                let variant_node = self.create_string_value("float")?;
                let mut variant_path = base_path.to_vec();
                variant_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "variant",
                )));
                self.layout.push(LayoutItem::Binding {
                    path: variant_path,
                    node: variant_node,
                    trailing_comment: None,
                });

                // field.range = "..."
                if let Some(range_str) =
                    self.float_bounds_to_range_string(&float.min, &float.max, float.range_style)
                {
                    let range_node = self.create_string_value(&range_str)?;
                    let mut range_path = base_path.to_vec();
                    range_path.push(SourcePathSegment::ident(Identifier::new_unchecked("range")));
                    self.layout.push(LayoutItem::Binding {
                        path: range_path,
                        node: range_node,
                        trailing_comment: None,
                    });
                }

                // field.multiple-of = N
                if let Some(mult) = &float.multiple_of {
                    let mult_node = self.create_float_value(*mult)?;
                    let mut mult_path = base_path.to_vec();
                    mult_path.push(SourcePathSegment::ident(Identifier::new_unchecked(
                        "multiple-of",
                    )));
                    self.layout.push(LayoutItem::Binding {
                        path: mult_path,
                        node: mult_node,
                        trailing_comment: None,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Convert a schema node to section items.
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
            SchemaNodeContent::Map(map) => {
                let map = map.clone();
                self.add_map_items(&map, &mut items)?;
            }
            SchemaNodeContent::Array(array) => {
                let array = array.clone();
                self.add_array_items(&array, &mut items)?;
            }
            SchemaNodeContent::Tuple(tuple) => {
                let tuple = tuple.clone();
                self.add_tuple_items(&tuple, &mut items)?;
            }
            SchemaNodeContent::Integer(int) => {
                let int = int.clone();
                self.add_integer_items(&int, &mut items)?;
            }
            SchemaNodeContent::Float(float) => {
                let float = float.clone();
                self.add_float_items(&float, &mut items)?;
            }
            SchemaNodeContent::Text(text) => {
                let text = text.clone();
                self.add_text_items(&text, &mut items)?;
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

    /// Convert a field schema to section items, including optional handling.
    fn convert_field_to_section_items(
        &mut self,
        node_id: SchemaNodeId,
        optional: bool,
    ) -> Result<Vec<LayoutItem>, ToSourceError> {
        let node = self.get_node(node_id)?.clone();
        let mut items = Vec::new();

        match &node.content {
            SchemaNodeContent::Record(record) => {
                let record = record.clone();
                // Add metadata first for structured types
                self.add_metadata_items(&node.metadata, &mut items)?;
                self.add_record_items(&record, &mut items)?;
            }
            SchemaNodeContent::Union(union) => {
                let union = union.clone();
                self.add_metadata_items(&node.metadata, &mut items)?;
                self.add_union_items(&union, &mut items)?;
            }
            SchemaNodeContent::Map(map) => {
                let map = map.clone();
                self.add_metadata_items(&node.metadata, &mut items)?;
                self.add_map_items(&map, &mut items)?;
            }
            SchemaNodeContent::Array(array) => {
                let array = array.clone();
                self.add_metadata_items(&node.metadata, &mut items)?;
                self.add_array_items(&array, &mut items)?;
            }
            SchemaNodeContent::Tuple(tuple) => {
                let tuple = tuple.clone();
                self.add_metadata_items(&node.metadata, &mut items)?;
                self.add_tuple_items(&tuple, &mut items)?;
            }
            _ => {
                // For simple types, value binding must come first
                let value_node = self.convert_schema_node(node_id)?;
                items.push(LayoutItem::Binding {
                    path: vec![],
                    node: value_node,
                    trailing_comment: None,
                });
                // Add metadata after value binding
                self.add_metadata_items(&node.metadata, &mut items)?;
            }
        }

        // Add $optional = true if this is an optional field
        // Per Eure grammar, bindings must come before sections,
        // so insert before the first section (if any)
        if optional {
            let optional_node = self.create_boolean_value(true)?;
            let optional_item = LayoutItem::Binding {
                path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                    "optional",
                ))],
                node: optional_node,
                trailing_comment: None,
            };

            // Find position of first section and insert before it
            let first_section_pos = items
                .iter()
                .position(|item| matches!(item, LayoutItem::Section { .. }));
            match first_section_pos {
                Some(pos) => items.insert(pos, optional_item),
                None => items.push(optional_item),
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
        _metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        // Check if we can use shorthand (metadata is output separately)
        if text.is_default_except_language() {
            let shorthand = match &text.language {
                Some(lang) => format!("text.{}", lang),
                None => "text".to_string(),
            };
            return self.create_inline_code(&shorthand);
        }

        // Expanded form: { $variant => "text", ... }
        let mut fields = Vec::new();

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
            let pattern_node = self.create_inline_code(pattern.as_str())?;
            fields.push(("pattern".to_string(), pattern_node));
        }

        let node_id = self.create_map_with_metadata(fields, &SchemaMetadata::default())?;
        self.add_variant_extension(node_id, "text")
    }

    fn convert_integer_schema(
        &mut self,
        int: &IntegerSchema,
        _metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        // Metadata is output separately, so use shorthand if constraints are default
        if int.is_default() {
            return self.create_inline_code("integer");
        }

        let mut fields = Vec::new();

        // Convert range if present
        if let Some(range_str) = self.bounds_to_range_string(&int.min, &int.max, int.range_style) {
            let range_node = self.create_string_value(&range_str)?;
            fields.push(("range".to_string(), range_node));
        }

        if let Some(mult) = &int.multiple_of {
            let mult_node = self.create_integer_value(mult.clone().try_into().unwrap_or(0))?;
            fields.push(("multiple-of".to_string(), mult_node));
        }

        let node_id = self.create_map_with_metadata(fields, &SchemaMetadata::default())?;
        self.add_variant_extension(node_id, "integer")
    }

    fn convert_float_schema(
        &mut self,
        float: &FloatSchema,
        _metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        // Metadata is output separately, so use shorthand if constraints are default
        if float.is_default() {
            return self.create_inline_code("float");
        }

        let mut fields = Vec::new();

        // Convert range if present
        if let Some(range_str) =
            self.float_bounds_to_range_string(&float.min, &float.max, float.range_style)
        {
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

        // Create map and add $variant = "float" extension
        let map_node = self.create_map_with_metadata(fields, &SchemaMetadata::default())?;
        self.add_variant_extension(map_node, "float")
    }

    fn convert_constrained_primitive(
        &mut self,
        type_name: &str,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        let fields = Vec::new();
        let map_node = self.create_map_with_metadata(fields, metadata)?;
        self.add_variant_extension(map_node, type_name)
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
        // Can't use shorthand if:
        // 1. prefer_inline is explicitly false
        // 2. Array has constraints (min/max/unique/contains/binding_style)
        // 3. Array has non-default metadata
        // 4. Item has non-default variant_repr (need item.$variant-repr = ...)
        let item_needs_section = self.item_needs_section_syntax(array.item)?;

        let can_use_shorthand = array.min_length.is_none()
            && array.max_length.is_none()
            && !array.unique
            && array.contains.is_none()
            && array.binding_style.is_none()
            && metadata == &SchemaMetadata::default()
            && !item_needs_section;

        // Use shorthand if:
        // - prefer_inline is Some(true), OR
        // - prefer_inline is None and shorthand is possible
        let use_shorthand = match array.prefer_inline {
            Some(true) => can_use_shorthand, // Prefer inline, use if possible
            Some(false) => false,            // Explicitly prefer expanded form
            None => can_use_shorthand,       // Auto-detect: use shorthand if possible
        };

        if use_shorthand {
            let item_node = self.convert_schema_node(array.item)?;
            return self.create_array_value(vec![item_node]);
        }

        // Expanded form
        let mut fields = Vec::new();

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

        // Create map and add $variant = "array" extension
        let map_node = self.create_map_with_metadata(fields, metadata)?;
        self.add_variant_extension(map_node, "array")
    }

    fn convert_map_schema(
        &mut self,
        map: &MapSchema,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        let mut fields = Vec::new();

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

        // Create map and add $variant = "map" extension
        let map_node = self.create_map_with_metadata(fields, metadata)?;
        self.add_variant_extension(map_node, "map")
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

            let elements_node = self.create_array_value(elements)?;
            fields.push(("elements".to_string(), elements_node));

            // Create map and add $variant = "tuple" extension
            let map_node = self.create_map_with_metadata(fields, metadata)?;
            self.add_variant_extension(map_node, "tuple")
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

            // Add $deny-untagged = true extension if this variant is in deny_untagged set
            if union.deny_untagged.contains(variant_name) {
                let doc = self.constructor.document_mut();
                let true_node =
                    doc.create_node(NodeValue::Primitive(PrimitiveValue::Bool(true)));
                let deny_untagged_key = Identifier::new_unchecked("deny-untagged");
                doc.node_mut(variant_node)
                    .extensions
                    .insert(deny_untagged_key, true_node);
            }

            fields.push((variant_name.clone(), variant_node));
        }

        // Create map and add $variant = "union" extension
        let map_node = self.create_map_with_metadata(fields, metadata)?;
        let map_node = self.add_variant_extension(map_node, "union")?;

        // Add $variant-repr extension if not default OR if it was explicitly specified
        if union.repr != VariantRepr::default() || union.repr_explicit {
            let repr_node = self.convert_variant_repr(&union.repr)?;
            let variant_repr_key = Identifier::new_unchecked("variant-repr");
            self.constructor
                .document_mut()
                .node_mut(map_node)
                .extensions
                .insert(variant_repr_key, repr_node);
        }

        Ok(map_node)
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

    /// Add metadata as bindings with a base path prefix.
    /// Used for simple fields where metadata appears as separate bindings like `field.$default = value`.
    fn add_metadata_as_bindings(
        &mut self,
        metadata: &SchemaMetadata,
        base_path: &[SourcePathSegment],
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // $description
        if let Some(desc) = &metadata.description {
            let desc_text = match desc {
                Description::String(s) => s.clone(),
                Description::Markdown(s) => s.clone(),
            };
            let desc_node = self.create_string_value(&desc_text)?;
            let mut path = base_path.to_vec();
            path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                "description",
            )));
            items.push(LayoutItem::Binding {
                path,
                node: desc_node,
                trailing_comment: None,
            });
        }

        // $deprecated
        if metadata.deprecated {
            let deprecated_node = self.create_boolean_value(true)?;
            let mut path = base_path.to_vec();
            path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                "deprecated",
            )));
            items.push(LayoutItem::Binding {
                path,
                node: deprecated_node,
                trailing_comment: None,
            });
        }

        // $default = value
        if let Some(default_doc) = &metadata.default {
            let default_node = self.copy_document_value(default_doc, default_doc.get_root_id())?;
            let mut path = base_path.to_vec();
            path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                "default",
            )));
            items.push(LayoutItem::Binding {
                path,
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
            let mut path = base_path.to_vec();
            path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                "examples",
            )));
            items.push(LayoutItem::Binding {
                path,
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
        // Check if any field uses section syntax - if so, ALL sections must use Block
        // to avoid ambiguity when parsing (sections without {} can "absorb" subsequent bindings)
        let has_multiple_fields = record.properties.len() > 1;
        let any_section_field = record
            .properties
            .values()
            .any(|f| self.should_use_section(f.schema).unwrap_or(false));
        let force_block = has_multiple_fields && any_section_field;

        // Per Eure grammar: Eure: [ ValueBinding ] { Binding } { Section }
        // Bindings must come before Sections. Collect separately and reorder.
        let mut binding_items = Vec::new();
        let mut section_items_vec = Vec::new();

        for (field_name, field_schema) in &record.properties {
            let ident: Identifier = field_name
                .parse()
                .unwrap_or_else(|_| Identifier::new_unchecked("invalid"));
            let path = vec![SourcePathSegment::ident(ident.clone())];

            // Check if this field should use section syntax
            if self.should_use_section(field_schema.schema)? {
                let section_items = self
                    .convert_field_to_section_items(field_schema.schema, field_schema.optional)?;
                // Use Block if nested sections exist OR if we need to disambiguate from siblings
                let body = if contains_nested_sections(&section_items) || force_block {
                    SectionBody::Block(section_items)
                } else {
                    SectionBody::Items(section_items)
                };
                section_items_vec.push(LayoutItem::Section {
                    path: path.clone(),
                    trailing_comment: None,
                    body,
                });
            } else {
                let field_node = self.convert_schema_node(field_schema.schema)?;

                binding_items.push(LayoutItem::Binding {
                    path: path.clone(),
                    node: field_node,
                    trailing_comment: None,
                });

                // Output $optional = true as a separate binding
                if field_schema.optional {
                    let optional_node = self.create_boolean_value(true)?;
                    let mut optional_path = path.clone();
                    optional_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                        "optional",
                    )));
                    binding_items.push(LayoutItem::Binding {
                        path: optional_path,
                        node: optional_node,
                        trailing_comment: None,
                    });
                }

                // Output metadata extensions for this field (like $default, $deprecated)
                let field_schema_node = self.get_node(field_schema.schema)?;
                let metadata = field_schema_node.metadata.clone();
                self.add_metadata_as_bindings(&metadata, &path, &mut binding_items)?;
            }

            // Output $ext-type extensions for this field
            let field_schema_node = self.get_node(field_schema.schema)?;
            let ext_types = field_schema_node.ext_types.clone();
            for (ext_name, ext_schema) in ext_types {
                // field.$ext-type.<ext-name> = <schema>
                let mut ext_path = path.clone();
                ext_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "ext-type",
                )));
                ext_path.push(SourcePathSegment::ident(ext_name.clone()));

                let ext_node = self.convert_schema_node(ext_schema.schema)?;
                binding_items.push(LayoutItem::Binding {
                    path: ext_path.clone(),
                    node: ext_node,
                    trailing_comment: None,
                });

                // field.$ext-type.<ext-name>.$optional = true (if optional)
                if ext_schema.optional {
                    let optional_node = self.create_boolean_value(true)?;
                    let mut optional_path = ext_path;
                    optional_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                        "optional",
                    )));
                    binding_items.push(LayoutItem::Binding {
                        path: optional_path,
                        node: optional_node,
                        trailing_comment: None,
                    });
                }
            }
        }

        // Add unknown-fields policy if not default
        match &record.unknown_fields {
            UnknownFieldsPolicy::Deny => {}
            UnknownFieldsPolicy::Allow => {
                let allow_node = self.create_string_value("allow")?;
                binding_items.push(LayoutItem::Binding {
                    path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                        "unknown-fields",
                    ))],
                    node: allow_node,
                    trailing_comment: None,
                });
            }
            UnknownFieldsPolicy::Schema(schema_id) => {
                let schema_node = self.convert_schema_node(*schema_id)?;
                binding_items.push(LayoutItem::Binding {
                    path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                        "unknown-fields",
                    ))],
                    node: schema_node,
                    trailing_comment: None,
                });
            }
        }

        // Add $flatten if present
        if !record.flatten.is_empty() {
            let flatten_refs: Vec<NodeId> = record
                .flatten
                .iter()
                .map(|&node_id| self.convert_schema_node(node_id))
                .collect::<Result<_, _>>()?;
            let flatten_array = self.create_array_value(flatten_refs)?;
            binding_items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                    "flatten",
                ))],
                node: flatten_array,
                trailing_comment: None,
            });
        }

        // Merge: bindings first, then sections (per Eure grammar)
        items.extend(binding_items);
        items.extend(section_items_vec);

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

        // Add $variant-repr if not default OR if it was explicitly specified
        if union.repr != VariantRepr::default() || union.repr_explicit {
            let repr_node = self.convert_variant_repr(&union.repr)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                    "variant-repr",
                ))],
                node: repr_node,
                trailing_comment: None,
            });
        }

        // Add variants under variants.* path
        for (variant_name, &variant_id) in &union.variants {
            let ident: Identifier = variant_name
                .parse()
                .unwrap_or_else(|_| Identifier::new_unchecked("invalid"));

            // Base path is variants.<variant_name>
            let base_path = vec![
                SourcePathSegment::ident(Identifier::new_unchecked("variants")),
                SourcePathSegment::ident(ident.clone()),
            ];

            let deny_untagged = union.deny_untagged.contains(variant_name);
            let variant_node = self.get_node(variant_id)?.clone();

            // Handle different variant types
            match &variant_node.content {
                SchemaNodeContent::Record(record) => {
                    // For record variants, output each field as flat binding
                    let record = record.clone();
                    for (field_name, field_schema) in &record.properties {
                        let field_ident: Identifier = field_name
                            .parse()
                            .unwrap_or_else(|_| Identifier::new_unchecked("invalid"));

                        let mut field_path = base_path.clone();
                        field_path.push(SourcePathSegment::ident(field_ident));

                        let field_node = self.convert_schema_node(field_schema.schema)?;
                        items.push(LayoutItem::Binding {
                            path: field_path.clone(),
                            node: field_node,
                            trailing_comment: None,
                        });

                        // Output $optional = true if needed
                        if field_schema.optional {
                            let optional_node = self.create_boolean_value(true)?;
                            let mut optional_path = field_path;
                            optional_path.push(SourcePathSegment::extension(
                                Identifier::new_unchecked("optional"),
                            ));
                            items.push(LayoutItem::Binding {
                                path: optional_path,
                                node: optional_node,
                                trailing_comment: None,
                            });
                        }
                    }
                }
                SchemaNodeContent::Union(inner_union) => {
                    // For nested union variants, output as section items with prefixed paths
                    let inner_union = inner_union.clone();
                    self.add_nested_union_items(&inner_union, &base_path, items)?;
                }
                _ => {
                    // For other variants, output as single binding
                    let variant_value = self.convert_schema_node(variant_id)?;
                    items.push(LayoutItem::Binding {
                        path: base_path.clone(),
                        node: variant_value,
                        trailing_comment: None,
                    });
                }
            }

            // Add $deny-untagged = true as separate binding if needed
            if deny_untagged {
                let true_node = self.create_boolean_value(true)?;
                let mut deny_path = base_path.clone();
                deny_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "deny-untagged",
                )));
                items.push(LayoutItem::Binding {
                    path: deny_path,
                    node: true_node,
                    trailing_comment: None,
                });
            }

            // Add $unambiguous = true as separate binding if needed
            if union.unambiguous.contains(variant_name) {
                let true_node = self.create_boolean_value(true)?;
                let mut unambiguous_path = base_path.clone();
                unambiguous_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "unambiguous",
                )));
                items.push(LayoutItem::Binding {
                    path: unambiguous_path,
                    node: true_node,
                    trailing_comment: None,
                });
            }

            // Output $ext-type extensions for this variant
            let ext_types = variant_node.ext_types.clone();
            for (ext_name, ext_schema) in ext_types {
                // variants.<variant_name>.$ext-type.<ext-name> = <schema>
                let mut ext_path = base_path.clone();
                ext_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "ext-type",
                )));
                ext_path.push(SourcePathSegment::ident(ext_name.clone()));

                let ext_node = self.convert_schema_node(ext_schema.schema)?;
                items.push(LayoutItem::Binding {
                    path: ext_path.clone(),
                    node: ext_node,
                    trailing_comment: None,
                });

                // variants.<variant_name>.$ext-type.<ext-name>.$optional = true (if optional)
                if ext_schema.optional {
                    let optional_node = self.create_boolean_value(true)?;
                    let mut optional_path = ext_path;
                    optional_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                        "optional",
                    )));
                    items.push(LayoutItem::Binding {
                        path: optional_path,
                        node: optional_node,
                        trailing_comment: None,
                    });
                }
            }
        }

        Ok(())
    }

    /// Add nested union items with prefixed paths.
    /// This preserves $variant, $variant-repr, and other extensions on nested unions.
    fn add_nested_union_items(
        &mut self,
        union: &UnionSchema,
        base_path: &[SourcePathSegment],
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // Add <base_path>.$variant = "union"
        let union_node = self.create_string_value("union")?;
        let mut variant_path = base_path.to_vec();
        variant_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
            "variant",
        )));
        items.push(LayoutItem::Binding {
            path: variant_path,
            node: union_node,
            trailing_comment: None,
        });

        // Add <base_path>.$variant-repr if not default OR if it was explicitly specified
        if union.repr != VariantRepr::default() || union.repr_explicit {
            let repr_node = self.convert_variant_repr(&union.repr)?;
            let mut repr_path = base_path.to_vec();
            repr_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                "variant-repr",
            )));
            items.push(LayoutItem::Binding {
                path: repr_path,
                node: repr_node,
                trailing_comment: None,
            });
        }

        // Add variants under <base_path>.variants.<variant_name>
        for (variant_name, &variant_id) in &union.variants {
            let ident: Identifier = variant_name
                .parse()
                .unwrap_or_else(|_| Identifier::new_unchecked("invalid"));

            // Path is <base_path>.variants.<variant_name>
            let mut variant_base_path = base_path.to_vec();
            variant_base_path.push(SourcePathSegment::ident(Identifier::new_unchecked(
                "variants",
            )));
            variant_base_path.push(SourcePathSegment::ident(ident.clone()));

            let deny_untagged = union.deny_untagged.contains(variant_name);
            let variant_node = self.get_node(variant_id)?.clone();

            match &variant_node.content {
                SchemaNodeContent::Record(record) => {
                    let record = record.clone();
                    for (field_name, field_schema) in &record.properties {
                        let field_ident: Identifier = field_name
                            .parse()
                            .unwrap_or_else(|_| Identifier::new_unchecked("invalid"));

                        let mut field_path = variant_base_path.clone();
                        field_path.push(SourcePathSegment::ident(field_ident));

                        let field_node = self.convert_schema_node(field_schema.schema)?;
                        items.push(LayoutItem::Binding {
                            path: field_path.clone(),
                            node: field_node,
                            trailing_comment: None,
                        });

                        if field_schema.optional {
                            let optional_node = self.create_boolean_value(true)?;
                            let mut optional_path = field_path;
                            optional_path.push(SourcePathSegment::extension(
                                Identifier::new_unchecked("optional"),
                            ));
                            items.push(LayoutItem::Binding {
                                path: optional_path,
                                node: optional_node,
                                trailing_comment: None,
                            });
                        }
                    }
                }
                SchemaNodeContent::Union(inner_union) => {
                    // Recursively handle nested unions
                    let inner_union = inner_union.clone();
                    self.add_nested_union_items(&inner_union, &variant_base_path, items)?;
                }
                _ => {
                    let variant_value = self.convert_schema_node(variant_id)?;
                    items.push(LayoutItem::Binding {
                        path: variant_base_path.clone(),
                        node: variant_value,
                        trailing_comment: None,
                    });
                }
            }

            if deny_untagged {
                let true_node = self.create_boolean_value(true)?;
                let mut deny_path = variant_base_path.clone();
                deny_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "deny-untagged",
                )));
                items.push(LayoutItem::Binding {
                    path: deny_path,
                    node: true_node,
                    trailing_comment: None,
                });
            }

            // Add $unambiguous = true if needed
            if union.unambiguous.contains(variant_name) {
                let true_node = self.create_boolean_value(true)?;
                let mut unambiguous_path = variant_base_path;
                unambiguous_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "unambiguous",
                )));
                items.push(LayoutItem::Binding {
                    path: unambiguous_path,
                    node: true_node,
                    trailing_comment: None,
                });
            }
        }

        Ok(())
    }

    fn add_map_items(
        &mut self,
        map: &MapSchema,
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // Add $variant = "map"
        let map_node = self.create_string_value("map")?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                "variant",
            ))],
            node: map_node,
            trailing_comment: None,
        });

        // Add key field
        let key_node = self.convert_schema_node(map.key)?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::ident(Identifier::new_unchecked("key"))],
            node: key_node,
            trailing_comment: None,
        });

        // Add value field
        let value_node = self.convert_schema_node(map.value)?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::ident(Identifier::new_unchecked("value"))],
            node: value_node,
            trailing_comment: None,
        });

        // Add optional size constraints
        if let Some(min) = map.min_size {
            let min_node = self.create_integer_value(min as i64)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "min-size",
                ))],
                node: min_node,
                trailing_comment: None,
            });
        }

        if let Some(max) = map.max_size {
            let max_node = self.create_integer_value(max as i64)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "max-size",
                ))],
                node: max_node,
                trailing_comment: None,
            });
        }

        Ok(())
    }

    fn add_array_items(
        &mut self,
        array: &ArraySchema,
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // Add $variant = "array"
        let array_node = self.create_string_value("array")?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                "variant",
            ))],
            node: array_node,
            trailing_comment: None,
        });

        // Add item field - check if item needs compound path bindings
        let item_schema = self.get_node(array.item)?;
        if let SchemaNodeContent::Union(union) = &item_schema.content
            && union.repr_explicit
        {
            // Use compound path bindings for union with explicit repr
            let base_path = vec![SourcePathSegment::ident(Identifier::new_unchecked("item"))];
            let union = union.clone();
            self.add_nested_union_items(&union, &base_path, items)?;
        } else {
            let item_node = self.convert_schema_node(array.item)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked("item"))],
                node: item_node,
                trailing_comment: None,
            });
        }

        // Add optional constraints
        if let Some(min) = array.min_length {
            let min_node = self.create_integer_value(min as i64)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "min-length",
                ))],
                node: min_node,
                trailing_comment: None,
            });
        }

        if let Some(max) = array.max_length {
            let max_node = self.create_integer_value(max as i64)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "max-length",
                ))],
                node: max_node,
                trailing_comment: None,
            });
        }

        if array.unique {
            let unique_node = self.create_boolean_value(true)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "unique",
                ))],
                node: unique_node,
                trailing_comment: None,
            });
        }

        if let Some(contains_id) = array.contains {
            let contains_schema = self.get_node(contains_id)?;
            if let SchemaNodeContent::Literal(lit_doc) = &contains_schema.content {
                // For literals, we need to output compound path bindings:
                // contains = <value>
                // contains.$variant = "literal"
                let lit_doc = lit_doc.clone();
                let value_node = self.copy_document_value(&lit_doc, lit_doc.get_root_id())?;
                items.push(LayoutItem::Binding {
                    path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                        "contains",
                    ))],
                    node: value_node,
                    trailing_comment: None,
                });

                let variant_node = self.create_string_value("literal")?;
                items.push(LayoutItem::Binding {
                    path: vec![
                        SourcePathSegment::ident(Identifier::new_unchecked("contains")),
                        SourcePathSegment::extension(Identifier::new_unchecked("variant")),
                    ],
                    node: variant_node,
                    trailing_comment: None,
                });
            } else {
                let contains_node = self.convert_schema_node(contains_id)?;
                items.push(LayoutItem::Binding {
                    path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                        "contains",
                    ))],
                    node: contains_node,
                    trailing_comment: None,
                });
            }
        }

        Ok(())
    }

    fn add_tuple_items(
        &mut self,
        tuple: &TupleSchema,
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // Add $variant = "tuple"
        let tuple_node = self.create_string_value("tuple")?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                "variant",
            ))],
            node: tuple_node,
            trailing_comment: None,
        });

        // Add elements as array
        let elements: Vec<NodeId> = tuple
            .elements
            .iter()
            .map(|&id| self.convert_schema_node(id))
            .collect::<Result<_, _>>()?;
        let elements_node = self.create_array_value(elements)?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                "elements",
            ))],
            node: elements_node,
            trailing_comment: None,
        });

        Ok(())
    }

    fn add_integer_items(
        &mut self,
        int: &IntegerSchema,
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // Add $variant = "integer"
        let variant_node = self.create_string_value("integer")?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                "variant",
            ))],
            node: variant_node,
            trailing_comment: None,
        });

        // Add range if present
        if let Some(range_str) = self.bounds_to_range_string(&int.min, &int.max, int.range_style) {
            let range_node = self.create_string_value(&range_str)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked("range"))],
                node: range_node,
                trailing_comment: None,
            });
        }

        // Add multiple-of if present
        if let Some(mult) = &int.multiple_of {
            let mult_node = self.create_integer_value(mult.clone().try_into().unwrap_or(0))?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "multiple-of",
                ))],
                node: mult_node,
                trailing_comment: None,
            });
        }

        Ok(())
    }

    fn add_float_items(
        &mut self,
        float: &FloatSchema,
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // Add $variant = "float"
        let variant_node = self.create_string_value("float")?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                "variant",
            ))],
            node: variant_node,
            trailing_comment: None,
        });

        // Add range if present
        if let Some(range_str) =
            self.float_bounds_to_range_string(&float.min, &float.max, float.range_style)
        {
            let range_node = self.create_string_value(&range_str)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked("range"))],
                node: range_node,
                trailing_comment: None,
            });
        }

        // Add multiple-of if present
        if let Some(mult) = &float.multiple_of {
            let mult_node = self.create_float_value(*mult)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "multiple-of",
                ))],
                node: mult_node,
                trailing_comment: None,
            });
        }

        Ok(())
    }

    fn add_text_items(
        &mut self,
        text: &TextSchema,
        items: &mut Vec<LayoutItem>,
    ) -> Result<(), ToSourceError> {
        // Add $variant = "text"
        let variant_node = self.create_string_value("text")?;
        items.push(LayoutItem::Binding {
            path: vec![SourcePathSegment::extension(Identifier::new_unchecked(
                "variant",
            ))],
            node: variant_node,
            trailing_comment: None,
        });

        // Add min-length if present
        if let Some(min) = text.min_length {
            let min_node = self.create_integer_value(min as i64)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "min-length",
                ))],
                node: min_node,
                trailing_comment: None,
            });
        }

        // Add max-length if present
        if let Some(max) = text.max_length {
            let max_node = self.create_integer_value(max as i64)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "max-length",
                ))],
                node: max_node,
                trailing_comment: None,
            });
        }

        // Add language if present
        if let Some(ref lang) = text.language {
            let lang_node = self.create_string_value(lang)?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "language",
                ))],
                node: lang_node,
                trailing_comment: None,
            });
        }

        // Add pattern if present (as inline code)
        if let Some(ref pattern) = text.pattern {
            let pattern_node = self.create_inline_code(pattern.as_str())?;
            items.push(LayoutItem::Binding {
                path: vec![SourcePathSegment::ident(Identifier::new_unchecked(
                    "pattern",
                ))],
                node: pattern_node,
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
            let ident: Identifier = field_name
                .parse()
                .unwrap_or_else(|_| Identifier::new_unchecked("invalid"));

            let mut path = base_path.clone();
            path.push(SourcePathSegment::ident(ident.clone()));

            let field_node = self.get_node(field_schema.schema)?.clone();

            // For nested records, recursively extend the path (compound path bindings)
            // This produces `root.user.name = \`text\`` instead of nested sections
            if let SchemaNodeContent::Record(ref nested_record) = field_node.content {
                let nested = nested_record.clone();
                self.convert_record_as_bindings(&nested, path.clone())?;

                // Output $optional for the nested record path if needed
                if field_schema.optional {
                    let optional_node = self.create_boolean_value(true)?;
                    let mut optional_path = path.clone();
                    optional_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                        "optional",
                    )));
                    self.layout.push(LayoutItem::Binding {
                        path: optional_path,
                        node: optional_node,
                        trailing_comment: None,
                    });
                }
            } else if self.is_constrained_primitive(field_schema.schema)? {
                // Constrained primitives use flat bindings: field.$variant = "text", field.min-length = 3
                self.output_constrained_primitive_as_bindings(field_schema.schema, &path)?;

                // Output $optional = true as a separate binding
                if field_schema.optional {
                    let optional_node = self.create_boolean_value(true)?;
                    let mut optional_path = path.clone();
                    optional_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                        "optional",
                    )));
                    self.layout.push(LayoutItem::Binding {
                        path: optional_path,
                        node: optional_node,
                        trailing_comment: None,
                    });
                }
            } else if self.should_use_section(field_schema.schema)? {
                // Use section for complex non-record types (unions, maps)
                let section_items = self
                    .convert_field_to_section_items(field_schema.schema, field_schema.optional)?;
                let body = if contains_nested_sections(&section_items) {
                    SectionBody::Block(section_items)
                } else {
                    SectionBody::Items(section_items)
                };
                self.layout.push(LayoutItem::Section {
                    path: path.clone(),
                    trailing_comment: None,
                    body,
                });
            } else {
                let value_node = self.convert_schema_node(field_schema.schema)?;

                self.layout.push(LayoutItem::Binding {
                    path: path.clone(),
                    node: value_node,
                    trailing_comment: None,
                });

                // Output $optional = true as a separate binding
                if field_schema.optional {
                    let optional_node = self.create_boolean_value(true)?;
                    let mut optional_path = path.clone();
                    optional_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                        "optional",
                    )));
                    self.layout.push(LayoutItem::Binding {
                        path: optional_path,
                        node: optional_node,
                        trailing_comment: None,
                    });
                }

                // Output metadata extensions for this field (like $default, $deprecated)
                let metadata = field_node.metadata.clone();
                let mut metadata_items = Vec::new();
                self.add_metadata_as_bindings(&metadata, &path, &mut metadata_items)?;
                for item in metadata_items {
                    self.layout.push(item);
                }
            }

            // Output $ext-type extensions for this field
            let ext_types = field_node.ext_types.clone();
            for (ext_name, ext_schema) in ext_types {
                // field.$ext-type.<ext-name> = <schema>
                let mut ext_path = path.clone();
                ext_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "ext-type",
                )));
                ext_path.push(SourcePathSegment::ident(ext_name.clone()));

                let ext_node = self.convert_schema_node(ext_schema.schema)?;
                self.layout.push(LayoutItem::Binding {
                    path: ext_path.clone(),
                    node: ext_node,
                    trailing_comment: None,
                });

                // field.$ext-type.<ext-name>.$optional = true (if optional)
                if ext_schema.optional {
                    let optional_node = self.create_boolean_value(true)?;
                    let mut optional_path = ext_path;
                    optional_path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                        "optional",
                    )));
                    self.layout.push(LayoutItem::Binding {
                        path: optional_path,
                        node: optional_node,
                        trailing_comment: None,
                    });
                }
            }
        }

        // Add unknown-fields policy if not default
        match &record.unknown_fields {
            UnknownFieldsPolicy::Deny => {}
            UnknownFieldsPolicy::Allow => {
                let allow_node = self.create_string_value("allow")?;
                let mut path = base_path.clone();
                path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "unknown-fields",
                )));
                self.layout.push(LayoutItem::Binding {
                    path,
                    node: allow_node,
                    trailing_comment: None,
                });
            }
            UnknownFieldsPolicy::Schema(schema_id) => {
                let schema_node = self.convert_schema_node(*schema_id)?;
                let mut path = base_path.clone();
                path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                    "unknown-fields",
                )));
                self.layout.push(LayoutItem::Binding {
                    path,
                    node: schema_node,
                    trailing_comment: None,
                });
            }
        }

        // Add $flatten if present
        if !record.flatten.is_empty() {
            let flatten_refs: Vec<NodeId> = record
                .flatten
                .iter()
                .map(|&node_id| self.convert_schema_node(node_id))
                .collect::<Result<_, _>>()?;
            let flatten_array = self.create_array_value(flatten_refs)?;
            let mut path = base_path;
            path.push(SourcePathSegment::extension(Identifier::new_unchecked(
                "flatten",
            )));
            self.layout.push(LayoutItem::Binding {
                path,
                node: flatten_array,
                trailing_comment: None,
            });
        }

        Ok(())
    }

    // =========================================================================
    // Helper Methods for Creating Document Nodes (using eure! macro)
    // =========================================================================

    fn create_inline_code(&mut self, code: &str) -> Result<NodeId, ToSourceError> {
        let code = code.to_string();
        self.with_new_node(|c| {
            c.bind_from(Text::inline_implicit(code)).unwrap();
        })
    }

    fn create_string_value(&mut self, s: &str) -> Result<NodeId, ToSourceError> {
        let s = s.to_string();
        self.with_new_node(|c| {
            c.bind_from(Text::plaintext(s)).unwrap();
        })
    }

    fn create_integer_value(&mut self, n: i64) -> Result<NodeId, ToSourceError> {
        self.with_new_node(|c| {
            c.bind_from(n).unwrap();
        })
    }

    fn create_float_value(&mut self, n: f64) -> Result<NodeId, ToSourceError> {
        self.with_new_node(|c| {
            c.bind_from(n).unwrap();
        })
    }

    fn create_boolean_value(&mut self, b: bool) -> Result<NodeId, ToSourceError> {
        self.with_new_node(|c| {
            c.bind_from(b).unwrap();
        })
    }

    fn create_array_value(&mut self, elements: Vec<NodeId>) -> Result<NodeId, ToSourceError> {
        self.with_new_node(|c| {
            c.bind_empty_array().unwrap();
            let arr_id = c.current_node_id();
            let doc = c.document_mut();
            for element in elements {
                let _ = doc.add_array_element(None, arr_id);
                if let Some(arr) = doc.node(arr_id).as_array()
                    && let Some(last_idx) = arr.len().checked_sub(1)
                    && let Some(last_id) = arr.get(last_idx)
                {
                    let element_content = doc.node(element).content.clone();
                    let element_extensions = doc.node(element).extensions.clone();
                    doc.node_mut(last_id).content = element_content;
                    doc.node_mut(last_id).extensions = element_extensions;
                }
            }
        })
    }

    fn create_tuple_value(&mut self, elements: Vec<NodeId>) -> Result<NodeId, ToSourceError> {
        self.with_new_node(|c| {
            c.bind_empty_tuple().unwrap();
            let tup_id = c.current_node_id();
            let doc = c.document_mut();
            for (index, element) in elements.into_iter().enumerate() {
                let _ = doc.add_tuple_element(index as u8, tup_id);
                if let Some(tup) = doc.node(tup_id).as_tuple()
                    && let Some(elem_id) = tup.get(index)
                {
                    let element_content = doc.node(element).content.clone();
                    let element_extensions = doc.node(element).extensions.clone();
                    doc.node_mut(elem_id).content = element_content;
                    doc.node_mut(elem_id).extensions = element_extensions;
                }
            }
        })
    }

    fn create_map_with_metadata(
        &mut self,
        fields: Vec<(String, NodeId)>,
        metadata: &SchemaMetadata,
    ) -> Result<NodeId, ToSourceError> {
        let map_id = self.with_new_node(|c| {
            c.bind_empty_map().unwrap();
            let map_id = c.current_node_id();
            let doc = c.document_mut();
            for (key, value_id) in fields {
                let object_key = ObjectKey::String(key);
                let _ = doc.add_map_child(object_key.clone(), map_id);
                if let Some(map) = doc.node(map_id).as_map()
                    && let Some(&child_id) = map.get(&object_key)
                {
                    let value_content = doc.node(value_id).content.clone();
                    let value_extensions = doc.node(value_id).extensions.clone();
                    doc.node_mut(child_id).content = value_content;
                    doc.node_mut(child_id).extensions = value_extensions;
                }
            }
        })?;

        // Add metadata as extensions if present
        self.add_metadata_extensions(map_id, metadata)?;

        Ok(map_id)
    }

    fn add_metadata_extensions(
        &mut self,
        node_id: NodeId,
        metadata: &SchemaMetadata,
    ) -> Result<(), ToSourceError> {
        let doc = self.constructor.document_mut();

        if let Some(desc) = &metadata.description {
            let desc_text = match desc {
                Description::String(s) => s.clone(),
                Description::Markdown(s) => s.clone(),
            };
            let desc_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
                Text::plaintext(desc_text),
            )));
            doc.node_mut(node_id)
                .extensions
                .insert(Identifier::new_unchecked("description"), desc_node);
        }

        if metadata.deprecated {
            let deprecated_node =
                doc.create_node(NodeValue::Primitive(PrimitiveValue::Bool(true)));
            doc.node_mut(node_id)
                .extensions
                .insert(Identifier::new_unchecked("deprecated"), deprecated_node);
        }

        if let Some(default_doc) = &metadata.default {
            let default_node = self.copy_document_value(default_doc, default_doc.get_root_id())?;
            self.constructor
                .document_mut()
                .node_mut(node_id)
                .extensions
                .insert(Identifier::new_unchecked("default"), default_node);
        }

        if let Some(examples) = &metadata.examples {
            let example_nodes: Vec<NodeId> = examples
                .iter()
                .map(|doc| self.copy_document_value(doc, doc.get_root_id()))
                .collect::<Result<_, _>>()?;
            let examples_node = self.create_array_value(example_nodes)?;
            self.constructor
                .document_mut()
                .node_mut(node_id)
                .extensions
                .insert(Identifier::new_unchecked("examples"), examples_node);
        }

        Ok(())
    }

    fn add_optional_extension(&mut self, node_id: NodeId) -> Result<NodeId, ToSourceError> {
        let doc = self.constructor.document_mut();
        let optional_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Bool(true)));
        doc.node_mut(node_id)
            .extensions
            .insert(Identifier::new_unchecked("optional"), optional_node);
        Ok(node_id)
    }

    fn add_variant_extension(
        &mut self,
        node_id: NodeId,
        variant: &str,
    ) -> Result<NodeId, ToSourceError> {
        let doc = self.constructor.document_mut();
        let variant_node = doc.create_node(NodeValue::Primitive(PrimitiveValue::Text(
            Text::plaintext(variant.to_string()),
        )));
        doc.node_mut(node_id)
            .extensions
            .insert(Identifier::new_unchecked("variant"), variant_node);
        Ok(node_id)
    }

    fn convert_variant_repr(&mut self, repr: &VariantRepr) -> Result<NodeId, ToSourceError> {
        match repr {
            VariantRepr::Untagged => self.create_string_value("untagged"),
            VariantRepr::External => self.create_string_value("external"),
            VariantRepr::Internal { tag } => {
                // { tag = "<tag>" } - simpler format that parser recognizes
                let tag_node = self.create_string_value(tag)?;
                let fields = vec![("tag".to_string(), tag_node)];
                self.create_map_with_metadata(fields, &SchemaMetadata::default())
            }
            VariantRepr::Adjacent { tag, content } => {
                // { tag = "<tag>", content = "<content>" }
                let tag_node = self.create_string_value(tag)?;
                let content_node = self.create_string_value(content)?;
                let fields = vec![
                    ("tag".to_string(), tag_node),
                    ("content".to_string(), content_node),
                ];
                self.create_map_with_metadata(fields, &SchemaMetadata::default())
            }
        }
    }

    fn copy_document_value(
        &mut self,
        source_doc: &EureDocument,
        source_id: NodeId,
    ) -> Result<NodeId, ToSourceError> {
        let source_node = source_doc.node(source_id);

        let new_id = match &source_node.content {
            NodeValue::Primitive(p) => self
                .constructor
                .document_mut()
                .create_node(NodeValue::Primitive(p.clone())),
            NodeValue::Array(arr) => {
                let arr = arr.clone();
                let array_id = self
                    .constructor
                    .document_mut()
                    .create_node(NodeValue::Array(Default::default()));
                for child_id in arr.iter() {
                    let child_copy = self.copy_document_value(source_doc, *child_id)?;
                    let doc = self.constructor.document_mut();
                    let _ = doc.add_array_element(None, array_id);
                    if let Some(arr) = doc.node(array_id).as_array()
                        && let Some(last_idx) = arr.len().checked_sub(1)
                        && let Some(last_id) = arr.get(last_idx)
                    {
                        let child_content = doc.node(child_copy).content.clone();
                        let child_extensions = doc.node(child_copy).extensions.clone();
                        doc.node_mut(last_id).content = child_content;
                        doc.node_mut(last_id).extensions = child_extensions;
                    }
                }
                array_id
            }
            NodeValue::Tuple(tup) => {
                let tup = tup.clone();
                let tuple_id = self
                    .constructor
                    .document_mut()
                    .create_node(NodeValue::Tuple(Default::default()));
                for (idx, child_id) in tup.iter().enumerate() {
                    let child_copy = self.copy_document_value(source_doc, *child_id)?;
                    let doc = self.constructor.document_mut();
                    let _ = doc.add_tuple_element(idx as u8, tuple_id);
                    if let Some(tup) = doc.node(tuple_id).as_tuple()
                        && let Some(elem_id) = tup.get(idx)
                    {
                        let child_content = doc.node(child_copy).content.clone();
                        let child_extensions = doc.node(child_copy).extensions.clone();
                        doc.node_mut(elem_id).content = child_content;
                        doc.node_mut(elem_id).extensions = child_extensions;
                    }
                }
                tuple_id
            }
            NodeValue::Map(map) => {
                let map = map.clone();
                let map_id = self
                    .constructor
                    .document_mut()
                    .create_node(NodeValue::Map(Default::default()));
                for (key, child_id) in map.iter() {
                    let child_copy = self.copy_document_value(source_doc, *child_id)?;
                    let doc = self.constructor.document_mut();
                    let _ = doc.add_map_child(key.clone(), map_id);
                    if let Some(map) = doc.node(map_id).as_map()
                        && let Some(&new_child_id) = map.get(key)
                    {
                        let child_content = doc.node(child_copy).content.clone();
                        let child_extensions = doc.node(child_copy).extensions.clone();
                        doc.node_mut(new_child_id).content = child_content;
                        doc.node_mut(new_child_id).extensions = child_extensions;
                    }
                }
                map_id
            }
            NodeValue::Hole(h) => self
                .constructor
                .document_mut()
                .create_node(NodeValue::Hole(h.clone())),
        };

        // Copy extensions
        let ext_keys: Vec<_> = source_node
            .extensions
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (ext_key, ext_value_id) in ext_keys {
            let ext_copy = self.copy_document_value(source_doc, ext_value_id)?;
            let node = self.constructor.document_mut().node_mut(new_id);
            node.extensions.insert(ext_key, ext_copy);
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
        style: RangeStyle,
    ) -> Option<String> {
        match style {
            RangeStyle::Rust => self.bounds_to_rust_range_string(min, max),
            RangeStyle::Interval => self.bounds_to_interval_string(min, max),
        }
    }

    fn bounds_to_rust_range_string<T: std::fmt::Display>(
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

    fn bounds_to_interval_string<T: std::fmt::Display>(
        &self,
        min: &Bound<T>,
        max: &Bound<T>,
    ) -> Option<String> {
        // Interval notation: [min, max], (min, max), [min, max), (min, max]
        let left_bracket = match min {
            Bound::Unbounded => "(",
            Bound::Inclusive(_) => "[",
            Bound::Exclusive(_) => "(",
        };
        let right_bracket = match max {
            Bound::Unbounded => ")",
            Bound::Inclusive(_) => "]",
            Bound::Exclusive(_) => ")",
        };
        let min_val = match min {
            Bound::Unbounded => String::new(),
            Bound::Inclusive(v) | Bound::Exclusive(v) => v.to_string(),
        };
        let max_val = match max {
            Bound::Unbounded => String::new(),
            Bound::Inclusive(v) | Bound::Exclusive(v) => v.to_string(),
        };

        if matches!(min, Bound::Unbounded) && matches!(max, Bound::Unbounded) {
            None
        } else {
            Some(format!(
                "{}{}, {}{}",
                left_bracket, min_val, max_val, right_bracket
            ))
        }
    }

    fn float_bounds_to_range_string(
        &self,
        min: &Bound<f64>,
        max: &Bound<f64>,
        style: RangeStyle,
    ) -> Option<String> {
        match style {
            RangeStyle::Rust => self.float_bounds_to_rust_range_string(min, max),
            RangeStyle::Interval => self.float_bounds_to_interval_string(min, max),
        }
    }

    fn float_bounds_to_rust_range_string(
        &self,
        min: &Bound<f64>,
        max: &Bound<f64>,
    ) -> Option<String> {
        match (min, max) {
            (Bound::Unbounded, Bound::Unbounded) => None,
            (Bound::Inclusive(min), Bound::Inclusive(max)) => {
                Some(format!("{}..={}", format_float(*min), format_float(*max)))
            }
            (Bound::Inclusive(min), Bound::Exclusive(max)) => {
                Some(format!("{}..{}", format_float(*min), format_float(*max)))
            }
            (Bound::Exclusive(min), Bound::Inclusive(max)) => {
                Some(format!("{}<..={}", format_float(*min), format_float(*max)))
            }
            (Bound::Exclusive(min), Bound::Exclusive(max)) => {
                Some(format!("{}<..{}", format_float(*min), format_float(*max)))
            }
            (Bound::Inclusive(min), Bound::Unbounded) => Some(format!("{}..", format_float(*min))),
            (Bound::Exclusive(min), Bound::Unbounded) => Some(format!("{}<..", format_float(*min))),
            (Bound::Unbounded, Bound::Inclusive(max)) => Some(format!("..={}", format_float(*max))),
            (Bound::Unbounded, Bound::Exclusive(max)) => Some(format!("..{}", format_float(*max))),
        }
    }

    fn float_bounds_to_interval_string(
        &self,
        min: &Bound<f64>,
        max: &Bound<f64>,
    ) -> Option<String> {
        // Interval notation: [min, max], (min, max), [min, max), (min, max]
        let left_bracket = match min {
            Bound::Unbounded => "(",
            Bound::Inclusive(_) => "[",
            Bound::Exclusive(_) => "(",
        };
        let right_bracket = match max {
            Bound::Unbounded => ")",
            Bound::Inclusive(_) => "]",
            Bound::Exclusive(_) => ")",
        };
        let min_val = match min {
            Bound::Unbounded => String::new(),
            Bound::Inclusive(v) | Bound::Exclusive(v) => format_float(*v),
        };
        let max_val = match max {
            Bound::Unbounded => String::new(),
            Bound::Inclusive(v) | Bound::Exclusive(v) => format_float(*v),
        };

        if matches!(min, Bound::Unbounded) && matches!(max, Bound::Unbounded) {
            None
        } else {
            Some(format!(
                "{}{}, {}{}",
                left_bracket, min_val, max_val, right_bracket
            ))
        }
    }
}

/// Format a float value, ensuring it always has a decimal point.
fn format_float(v: f64) -> String {
    let s = v.to_string();
    // If the float has no decimal point (e.g., "0" instead of "0.0"), add ".0"
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        format!("{}.0", s)
    } else {
        s
    }
}

/// Check if layout items contain nested sections.
/// When true, the containing section should use Block syntax ({ }) to preserve nesting.
fn contains_nested_sections(items: &[LayoutItem]) -> bool {
    items
        .iter()
        .any(|item| matches!(item, LayoutItem::Section { .. }))
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
