//! Schema extraction from EureDocument
//!
//! This module provides functionality to extract schema information from
//! EURE documents using the new EureDocument structure.

use crate::identifiers;
use crate::schema::*;
use crate::utils::path_to_display_string;
use eure_tree::document::{DocumentKey, EureDocument, Node, NodeValue};
use eure_value::value::{KeyCmpValue, Path};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Errors that can occur during schema extraction
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("Invalid type path: {0}")]
    InvalidTypePath(String),

    #[error("Invalid field in schema: {0}")]
    InvalidField(String),

    #[error("Conflicting type definitions for: {0}")]
    ConflictingTypes(String),

    #[error("Invalid variant definition")]
    InvalidVariant,

    #[error("Invalid rename rule: {0}")]
    InvalidRenameRule(String),
}

/// Extract a schema from an EureDocument
pub fn document_to_schema(doc: &EureDocument) -> Result<DocumentSchema, SchemaError> {
    let mut schema = DocumentSchema::default();
    let mut builder = SchemaBuilder::new();

    // Process the root node
    builder.process_node(doc, doc.get_root(), &[])?;

    // Extract built schemas
    schema.types = builder.types;
    schema.root = ObjectSchema {
        fields: builder.root_fields,
        additional_properties: None,
    };
    schema.cascade_types = builder.cascade_types;

    // Check for special root-level extensions
    let root = doc.get_root();

    // Handle schema ref
    if let Some(schema_node_id) = root.extensions.get(&identifiers::SCHEMA) {
        let schema_node = doc.get_node(*schema_node_id);
        if let NodeValue::String {
            value: schema_ref, ..
        } = &schema_node.content
        {
            schema.schema_ref = Some(schema_ref.clone());
        }
    }

    // Handle root-level cascade type
    if let Some(cascade_node_id) = root.extensions.get(&identifiers::CASCADE_TYPE) {
        let cascade_node = doc.get_node(*cascade_node_id);
        if let NodeValue::Path { value: path, .. } = &cascade_node.content
            && let Some(cascade_type) = Type::from_path_segments(&path.0)
        {
            schema.cascade_types.insert(Path::root(), cascade_type);
        }
    }

    // Handle global serde options
    // Check for $rename extension
    if let Some(rename_node_id) = root.extensions.get(&identifiers::RENAME) {
        let rename_node = doc.get_node(*rename_node_id);
        if let NodeValue::String { value: rename, .. } = &rename_node.content {
            schema.serde_options.rename = Some(rename.clone());
        }
    }

    // Check for $rename-all extension
    if let Some(rename_all_node_id) = root.extensions.get(&identifiers::RENAME_ALL) {
        let rename_all_node = doc.get_node(*rename_all_node_id);
        if let NodeValue::String {
            value: rename_all, ..
        } = &rename_all_node.content
        {
            schema.serde_options.rename_all = Some(rename_all.parse()?);
        }
    }

    Ok(schema)
}

/// Check if a node has any schema-defining extensions
fn has_schema_extensions(node: &Node) -> bool {
    node.extensions.iter().any(|(ext, _)| {
        matches!(
            ext.as_ref(),
            "type"
                | "optional"
                | "min"
                | "max"
                | "pattern"
                | "min-length"
                | "max-length"
                | "length"
                | "range"
                | "union"
                | "variants"
                | "cascade-type"
                | "array"
                | "enum"
                | "values"
                | "default"
                | "unique"
                | "contains"
        )
    })
}

/// Check if a Node represents a pure schema (no data content)
pub fn is_pure_schema_node(doc: &EureDocument, node: &Node) -> bool {
    match &node.content {
        NodeValue::Map { entries, .. } => {
            // Check all entries
            for (key, node_id) in entries {
                match key {
                    DocumentKey::MetaExtension(_) => {
                        // Meta-extension keys are OK for schemas
                        continue;
                    }
                    DocumentKey::Ident(_) | DocumentKey::Value(_) => {
                        let child_node = doc.get_node(*node_id);
                        if !is_schema_or_nested_schema_node(doc, child_node) {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            true
        }
        _ => {
            // Other content types with extensions might be schema definitions
            !node.extensions.is_empty()
        }
    }
}

fn is_schema_or_nested_schema_node(doc: &EureDocument, node: &Node) -> bool {
    // A node is a schema node if:
    // 1. It has schema-defining extensions (other than just $type)
    // 2. OR it's a map that only contains other schema nodes

    // Check if this node has non-type schema extensions
    let has_schema_extensions = node.extensions.iter().any(|(ext, _)| {
        ext.as_ref() == "optional"
            || ext.as_ref() == "min"
            || ext.as_ref() == "max"
            || ext.as_ref() == "pattern"
            || ext.as_ref() == "values"
            || ext.as_ref() == "length"
            || ext.as_ref() == "range"
            || ext.as_ref() == "union"
            || ext.as_ref() == "variants"
            || ext.as_ref() == "cascade-type"
    });

    if has_schema_extensions {
        return true;
    }

    // If it only has a $type extension, check if it's a map containing only schema nodes
    match &node.content {
        NodeValue::Map { entries, .. } => {
            // If the map has non-extension entries, it's likely data
            let has_data_entries = entries
                .iter()
                .any(|(key, _)| matches!(key, DocumentKey::Ident(_) | DocumentKey::Value(_)));

            if has_data_entries {
                // Check if all non-extension entries are schema nodes
                for (key, node_id) in entries {
                    match key {
                        DocumentKey::MetaExtension(_) => continue,
                        DocumentKey::Ident(_) | DocumentKey::Value(_) => {
                            let child_node = doc.get_node(*node_id);
                            if !is_pure_schema_node(doc, child_node) {
                                return false;
                            }
                        }
                        _ => {}
                    }
                }
                true
            } else {
                // Map with only extensions is a schema node
                true
            }
        }
        _ => {
            // Non-map nodes with only $type are data nodes
            false
        }
    }
}

/// Helper to build schemas from EureDocument
struct SchemaBuilder {
    types: IndexMap<KeyCmpValue, FieldSchema>,
    root_fields: IndexMap<KeyCmpValue, FieldSchema>,
    cascade_types: HashMap<Path, Type>,
}

impl SchemaBuilder {
    fn new() -> Self {
        Self {
            types: IndexMap::new(),
            root_fields: IndexMap::new(),
            cascade_types: HashMap::new(),
        }
    }

    /// Process a node at a given path
    fn process_node(
        &mut self,
        doc: &EureDocument,
        node: &Node,
        path: &[&str],
    ) -> Result<(), SchemaError> {
        // Check if we're in the types extension namespace
        if !path.is_empty() && path[0] == "types" {
            self.process_types_node(doc, node)?;
            return Ok(());
        }

        // Process extensions first
        for (ext_name, ext_node_id) in &node.extensions {
            let ext_node = doc.get_node(*ext_node_id);

            match ext_name.as_ref() {
                "types" => {
                    // Process type definitions
                    self.process_types_node(doc, ext_node)?;
                }
                "cascade-type" => {
                    // Handle cascade-type extensions which can define nested field schemas
                    // For example: $cascade-type.items.$array = .$types.WithCascade
                    if let NodeValue::Map { entries, .. } = &ext_node.content {
                        // Process nested cascade type definitions
                        for (key, node_id) in entries {
                            if let DocumentKey::Ident(field_name) = key {
                                let field_node = doc.get_node(*node_id);
                                // Check if this field has an $array extension
                                if let Some(array_node_id) =
                                    field_node.extensions.get(&identifiers::ARRAY)
                                {
                                    let array_node = doc.get_node(*array_node_id);
                                    if let NodeValue::Path {
                                        value: type_path, ..
                                    } = &array_node.content
                                        && let Some(elem_type) =
                                            Type::from_path_segments(&type_path.0)
                                    {
                                        // Create an array field with the specified element type
                                        self.root_fields.insert(
                                            KeyCmpValue::String(field_name.to_string()),
                                            FieldSchema {
                                                type_expr: Type::Array(Box::new(elem_type)),
                                                optional: false,
                                                ..Default::default()
                                            },
                                        );
                                    }
                                } else if let NodeValue::Path {
                                    value: type_path, ..
                                } = &field_node.content
                                {
                                    // Direct field type: $cascade-type.field = .string
                                    if let Some(field_type) = Type::from_path_segments(&type_path.0)
                                    {
                                        self.root_fields.insert(
                                            KeyCmpValue::String(field_name.to_string()),
                                            FieldSchema {
                                                type_expr: field_type,
                                                optional: false,
                                                ..Default::default()
                                            },
                                        );
                                    }
                                } else {
                                    // Recursively handle nested objects if needed
                                    if let Some(field_schema) =
                                        Self::extract_field_schema_from_node(
                                            doc,
                                            field_name.as_ref(),
                                            field_node,
                                        )?
                                    {
                                        self.root_fields.insert(
                                            KeyCmpValue::String(field_name.to_string()),
                                            field_schema,
                                        );
                                    }
                                }
                            }
                        }
                    } else if let NodeValue::Path {
                        value: _type_path, ..
                    } = &ext_node.content
                    {
                        // Direct cascade type: $cascade-type = .string
                        // This is handled by setting cascade_type on the root ObjectSchema
                    }
                }
                _ => {
                    // Other extensions might be field schemas
                    if path.is_empty()
                        && let Some(schema) =
                            Self::extract_field_schema_from_node(doc, ext_name.as_ref(), ext_node)?
                    {
                        // Store extension schemas directly by their identifier
                        self.root_fields
                            .insert(KeyCmpValue::MetaExtension(ext_name.clone()), schema);
                    }
                }
            }
        }

        // Process map content
        if let NodeValue::Map { entries, .. } = &node.content {
            for (key, node_id) in entries {
                let child_node = doc.get_node(*node_id);

                match key {
                    DocumentKey::Ident(ident) => {
                        if path.is_empty() {
                            // Check if this is the types namespace
                            if ident.as_ref() == "types" {
                                // Process as types namespace
                                self.process_types_node(doc, child_node)?;
                            } else {
                                // Regular field at root level
                                // Always check if there's a schema for this field, regardless of whether it has data
                                if let Some(field_schema) = Self::extract_field_schema_from_node(
                                    doc,
                                    ident.as_ref(),
                                    child_node,
                                )? {
                                    self.root_fields.insert(
                                        KeyCmpValue::String(ident.to_string()),
                                        field_schema,
                                    );
                                } else if has_schema_extensions(child_node) {
                                    // If the node has schema extensions but extract_field_schema_from_node returned None,
                                    // it might be because the node also has data content
                                    // In this case, we should still extract the schema
                                    let mut field_schema = FieldSchema::default();
                                    if self.extract_schema_from_extensions(
                                        doc,
                                        child_node,
                                        &mut field_schema,
                                    )? {
                                        self.root_fields.insert(
                                            KeyCmpValue::String(ident.to_string()),
                                            field_schema,
                                        );
                                    }
                                }
                            }
                        } else {
                            // For nested paths, recursively process
                            let mut new_path = path.to_vec();
                            new_path.push(ident.as_ref());
                            self.process_node(doc, child_node, &new_path)?;
                        }
                    }
                    DocumentKey::MetaExtension(meta_ext_ident) => {
                        // Meta-extensions define schemas for extensions
                        if let Some(field_schema) = Self::extract_field_schema_from_node(
                            doc,
                            meta_ext_ident.as_ref(),
                            child_node,
                        )? {
                            // Store as meta-extension schema
                            self.root_fields.insert(
                                KeyCmpValue::MetaExtension(meta_ext_ident.clone()),
                                field_schema,
                            );
                        }
                    }
                    DocumentKey::Value(val) => {
                        // Handle quoted field names like "$variant", "a.b.c", etc.
                        if path.is_empty()
                            && let KeyCmpValue::String(field_name) = val
                            && let Some(field_schema) =
                                Self::extract_field_schema_from_node(doc, field_name, child_node)?
                        {
                            self.root_fields
                                .insert(KeyCmpValue::String(field_name.clone()), field_schema);
                        }
                    }
                    _ => {} // Skip other key types
                }
            }
        }

        Ok(())
    }

    /// Process nodes in the types extension namespace
    fn process_types_node(&mut self, doc: &EureDocument, node: &Node) -> Result<(), SchemaError> {
        if let NodeValue::Map { entries, .. } = &node.content {
            for (key, node_id) in entries {
                if let DocumentKey::Ident(type_name) = key {
                    let type_node = doc.get_node(*node_id);
                    if let Some(type_schema) =
                        self.extract_type_definition(doc, type_name.as_ref(), type_node)?
                    {
                        self.types
                            .insert(KeyCmpValue::String(type_name.to_string()), type_schema);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract type definition from a node
    fn extract_type_definition(
        &self,
        doc: &EureDocument,
        type_name: &str,
        node: &Node,
    ) -> Result<Option<FieldSchema>, SchemaError> {
        // A type definition can be:
        // 1. A direct type path: Point = .tuple
        // 2. An object with extensions: User = { $$type = .object, ... }
        // 3. A variant definition with $variants

        match &node.content {
            NodeValue::Path { value: path, .. } => {
                // Direct type reference
                if let Some(type_expr) = Type::from_path_segments(&path.0) {
                    Ok(Some(FieldSchema {
                        type_expr,
                        ..Default::default()
                    }))
                } else {
                    Err(SchemaError::InvalidTypePath(path_to_display_string(path)))
                }
            }
            NodeValue::Map { .. } => {
                // Check if this is a variant type
                if let Some(variants_node_id) = node.extensions.get(&identifiers::VARIANTS) {
                    let variants_node = doc.get_node(*variants_node_id);
                    let variants = self.extract_all_variants_from_node(doc, variants_node)?;

                    // Determine variant representation
                    let mut repr = VariantRepr::Tagged;
                    if let Some(repr_node_id) = node.extensions.get(&identifiers::VARIANT_REPR) {
                        let repr_node = doc.get_node(*repr_node_id);
                        repr = match &repr_node.content {
                            NodeValue::String {
                                value: repr_str, ..
                            } => match repr_str.as_str() {
                                "untagged" => VariantRepr::Untagged,
                                "external" => VariantRepr::Tagged,
                                _ => VariantRepr::Tagged,
                            },
                            NodeValue::Map { entries, .. } => {
                                // Parse object notation for internally/adjacently tagged
                                let tag_key = DocumentKey::Ident(identifiers::TAG.clone());
                                let content_key = DocumentKey::Ident(identifiers::CONTENT.clone());

                                let tag_value = entries
                                    .iter()
                                    .find(|(k, _)| k == &tag_key)
                                    .and_then(|(_, id)| {
                                        let node = doc.get_node(*id);
                                        if let NodeValue::String { value, .. } = &node.content {
                                            Some(value.clone())
                                        } else {
                                            None
                                        }
                                    });
                                let content_value = entries
                                    .iter()
                                    .find(|(k, _)| k == &content_key)
                                    .and_then(|(_, id)| {
                                        let node = doc.get_node(*id);
                                        if let NodeValue::String { value, .. } = &node.content {
                                            Some(value.clone())
                                        } else {
                                            None
                                        }
                                    });

                                match (tag_value, content_value) {
                                    (Some(tag), Some(content)) => VariantRepr::AdjacentlyTagged {
                                        tag: KeyCmpValue::String(tag),
                                        content: KeyCmpValue::String(content),
                                    },
                                    (Some(tag), None) => VariantRepr::InternallyTagged {
                                        tag: KeyCmpValue::String(tag),
                                    },
                                    _ => VariantRepr::Tagged,
                                }
                            }
                            _ => VariantRepr::Tagged,
                        };
                    }

                    return Ok(Some(FieldSchema {
                        type_expr: Type::Variants(VariantSchema {
                            variants,
                            representation: repr,
                        }),
                        ..Default::default()
                    }));
                }

                // Check if this is an object type definition with fields
                let mut fields = IndexMap::new();
                let mut has_fields = false;

                if let NodeValue::Map { entries, .. } = &node.content {
                    for (key, field_node_id) in entries {
                        match key {
                            DocumentKey::Ident(field_name) => {
                                let field_node = doc.get_node(*field_node_id);
                                if let Some(field_schema) = Self::extract_field_schema_from_node(
                                    doc,
                                    field_name.as_ref(),
                                    field_node,
                                )? {
                                    fields.insert(
                                        KeyCmpValue::String(field_name.to_string()),
                                        field_schema,
                                    );
                                    has_fields = true;
                                }
                            }
                            DocumentKey::Value(val) => {
                                // Handle quoted field names in type definitions
                                if let KeyCmpValue::String(field_name) = val
                                    && let field_node = doc.get_node(*field_node_id)
                                    && let Some(field_schema) =
                                        Self::extract_field_schema_from_node(
                                            doc, field_name, field_node,
                                        )?
                                {
                                    fields.insert(
                                        KeyCmpValue::String(field_name.clone()),
                                        field_schema,
                                    );
                                    has_fields = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                if has_fields {
                    // This is an object type with fields
                    let obj_schema = ObjectSchema {
                        fields,
                        additional_properties: None,
                    };
                    Ok(Some(FieldSchema {
                        type_expr: Type::Object(obj_schema),
                        ..Default::default()
                    }))
                } else {
                    // Otherwise, extract as regular field schema
                    Self::extract_field_schema_from_node(doc, type_name, node)
                }
            }
            NodeValue::Tuple { children, .. } => {
                // Handle tuple type definitions like: Point = (.number, .number)
                let mut element_types = Vec::new();

                for child_id in children {
                    let child_node = doc.get_node(*child_id);
                    // Each element should be a type path
                    match &child_node.content {
                        NodeValue::Path { value: path, .. } => {
                            if let Some(element_type) = Type::from_path_segments(&path.0) {
                                element_types.push(element_type);
                            } else {
                                return Err(SchemaError::InvalidTypePath(path_to_display_string(
                                    path,
                                )));
                            }
                        }
                        _ => {
                            // For now, only support type paths in tuple definitions
                            return Err(SchemaError::InvalidField(format!(
                                "Tuple type elements must be type paths, got {:?}",
                                child_node.content
                            )));
                        }
                    }
                }

                Ok(Some(FieldSchema {
                    type_expr: Type::Tuple(element_types),
                    ..Default::default()
                }))
            }
            _ => Ok(None),
        }
    }

    /// Extract schema information from node extensions only
    fn extract_schema_from_extensions(
        &self,
        doc: &EureDocument,
        node: &Node,
        schema: &mut FieldSchema,
    ) -> Result<bool, SchemaError> {
        let mut has_schema = false;

        // Check extensions for schema information
        for (ext_name, ext_node_id) in &node.extensions {
            let ext_node = doc.get_node(*ext_node_id);

            match ext_name.as_ref() {
                "type" => {
                    match &ext_node.content {
                        NodeValue::Path { value: path, .. } => {
                            if let Some(type_expr) = Type::from_path_segments(&path.0) {
                                schema.type_expr = type_expr;
                                has_schema = true;
                            }
                        }
                        NodeValue::Tuple { children, .. } => {
                            // Handle tuple type like matrix.$type = (.number, .number)
                            let mut element_types = Vec::new();
                            let mut valid = true;

                            for child_id in children {
                                let child_node = doc.get_node(*child_id);
                                match &child_node.content {
                                    NodeValue::Path { value: path, .. } => {
                                        if let Some(element_type) =
                                            Type::from_path_segments(&path.0)
                                        {
                                            element_types.push(element_type);
                                        } else {
                                            valid = false;
                                            break;
                                        }
                                    }
                                    _ => {
                                        valid = false;
                                        break;
                                    }
                                }
                            }

                            if valid && !element_types.is_empty() {
                                schema.type_expr = Type::Tuple(element_types);
                                has_schema = true;
                            }
                        }
                        _ => {}
                    }
                }
                "optional" => {
                    if let NodeValue::Bool { value: b, .. } = &ext_node.content {
                        schema.optional = *b;
                        has_schema = true;
                    }
                }
                "min" => {
                    if let Some(min_value) = Self::extract_f64_from_node(&ext_node.content) {
                        if let Some((ref mut min, _)) = schema.constraints.range {
                            *min = Some(min_value);
                        } else {
                            schema.constraints.range = Some((Some(min_value), None));
                        }
                        has_schema = true;
                    }
                }
                "max" => {
                    if let Some(max_value) = Self::extract_f64_from_node(&ext_node.content) {
                        if let Some((_, ref mut max)) = schema.constraints.range {
                            *max = Some(max_value);
                        } else {
                            schema.constraints.range = Some((None, Some(max_value)));
                        }
                        has_schema = true;
                    }
                }
                "min-length" => {
                    if let Some(min_len) = Self::extract_usize_from_node(&ext_node.content) {
                        if let Some((ref mut min, _)) = schema.constraints.length {
                            *min = Some(min_len);
                        } else {
                            schema.constraints.length = Some((Some(min_len), None));
                        }
                        has_schema = true;
                    }
                }
                "max-length" => {
                    if let Some(max_len) = Self::extract_usize_from_node(&ext_node.content) {
                        if let Some((_, ref mut max)) = schema.constraints.length {
                            *max = Some(max_len);
                        } else {
                            schema.constraints.length = Some((None, Some(max_len)));
                        }
                        has_schema = true;
                    }
                }
                "pattern" => {
                    if let NodeValue::String { value: s, .. } = &ext_node.content {
                        schema.constraints.pattern = Some(s.clone());
                        has_schema = true;
                    }
                }
                // Add other extension handlers as needed...
                _ => {}
            }
        }

        Ok(has_schema)
    }

    /// Extract field schema from a node
    fn extract_field_schema_from_node(
        doc: &EureDocument,
        _field_name: &str,
        node: &Node,
    ) -> Result<Option<FieldSchema>, SchemaError> {
        let mut schema = FieldSchema::default();
        let mut has_schema = false;

        // Check extensions for schema information
        for (ext_name, ext_node_id) in &node.extensions {
            let ext_node = doc.get_node(*ext_node_id);

            match ext_name.as_ref() {
                "type" => {
                    match &ext_node.content {
                        NodeValue::Path { value: path, .. } => {
                            if let Some(type_expr) = Type::from_path_segments(&path.0) {
                                schema.type_expr = type_expr;
                                has_schema = true;
                            }
                        }
                        NodeValue::Tuple { children, .. } => {
                            // Handle tuple type like matrix.$type = (.number, .number)
                            let mut element_types = Vec::new();
                            let mut valid = true;

                            for child_id in children {
                                let child_node = doc.get_node(*child_id);
                                match &child_node.content {
                                    NodeValue::Path { value: path, .. } => {
                                        if let Some(element_type) =
                                            Type::from_path_segments(&path.0)
                                        {
                                            element_types.push(element_type);
                                        } else {
                                            valid = false;
                                            break;
                                        }
                                    }
                                    _ => {
                                        valid = false;
                                        break;
                                    }
                                }
                            }

                            if valid && !element_types.is_empty() {
                                schema.type_expr = Type::Tuple(element_types);
                                has_schema = true;
                            }
                        }
                        _ => {}
                    }
                }
                "optional" => {
                    if let NodeValue::Bool { value: b, .. } = &ext_node.content {
                        schema.optional = *b;
                        has_schema = true;
                    }
                }
                "min" => {
                    if let Some(min_value) = Self::extract_f64_from_node(&ext_node.content) {
                        if let Some((ref mut min, _)) = schema.constraints.range {
                            *min = Some(min_value);
                        } else {
                            schema.constraints.range = Some((Some(min_value), None));
                        }
                        has_schema = true;
                    }
                }
                "max" => {
                    if let Some(max_value) = Self::extract_f64_from_node(&ext_node.content) {
                        if let Some((_, ref mut max)) = schema.constraints.range {
                            *max = Some(max_value);
                        } else {
                            schema.constraints.range = Some((None, Some(max_value)));
                        }
                        has_schema = true;
                    }
                }
                "min-length" => {
                    if let Some(min_len) = Self::extract_usize_from_node(&ext_node.content) {
                        if let Some((ref mut min, _)) = schema.constraints.length {
                            *min = Some(min_len);
                        } else {
                            schema.constraints.length = Some((Some(min_len), None));
                        }
                        has_schema = true;
                    }
                }
                "max-length" => {
                    if let Some(max_len) = Self::extract_usize_from_node(&ext_node.content) {
                        if let Some((_, ref mut max)) = schema.constraints.length {
                            *max = Some(max_len);
                        } else {
                            schema.constraints.length = Some((None, Some(max_len)));
                        }
                        has_schema = true;
                    }
                }
                "pattern" => {
                    if let NodeValue::String { value: s, .. } = &ext_node.content {
                        schema.constraints.pattern = Some(s.clone());
                        has_schema = true;
                    }
                }
                "range" => {
                    // Handle $range = (min, max)
                    if let NodeValue::Tuple { children, .. } = &ext_node.content
                        && children.len() == 2
                    {
                        let min_node = doc.get_node(children[0]);
                        let max_node = doc.get_node(children[1]);
                        let min_value = Self::extract_f64_from_node(&min_node.content);
                        let max_value = Self::extract_f64_from_node(&max_node.content);
                        schema.constraints.range = Some((min_value, max_value));
                        has_schema = true;
                    }
                }
                "length" => {
                    // Handle $length = (min, max)
                    if let NodeValue::Tuple { children, .. } = &ext_node.content
                        && children.len() == 2
                    {
                        let min_node = doc.get_node(children[0]);
                        let max_node = doc.get_node(children[1]);
                        let min_value = Self::extract_usize_from_node(&min_node.content);
                        let max_value = Self::extract_usize_from_node(&max_node.content);
                        schema.constraints.length = Some((min_value, max_value));
                        has_schema = true;
                    }
                }
                "values" => {
                    // Note: values/enum constraint is not in the current Constraints struct
                    // This would need to be added if enum validation is required
                    has_schema = true;
                }
                "union" => {
                    // Handle union types: $union = [.string, .number]
                    if let NodeValue::Array { children, .. } = &ext_node.content {
                        let mut union_types = Vec::new();
                        for child_id in children {
                            let child_node = doc.get_node(*child_id);
                            if let NodeValue::Path { value: path, .. } = &child_node.content
                                && let Some(union_type) = Type::from_path_segments(&path.0)
                            {
                                union_types.push(union_type);
                            }
                        }
                        if !union_types.is_empty() {
                            schema.type_expr = Type::Union(union_types);
                            has_schema = true;
                        }
                    }
                }
                "array" => {
                    // Handle array extension: field.$array = .type or field.$array = { inline object }
                    match &ext_node.content {
                        NodeValue::Path { value: path, .. } => {
                            // Array with type reference: field.$array = .$types.Item
                            if let Some(elem_type) = Type::from_path_segments(&path.0) {
                                schema.type_expr = Type::Array(Box::new(elem_type));
                                has_schema = true;
                            }
                        }
                        NodeValue::Map { .. } => {
                            // Array with inline object: field.$array = { id = .number, name = .string }
                            let mut fields = IndexMap::new();
                            if let NodeValue::Map { entries, .. } = &ext_node.content {
                                for (key, field_node_id) in entries {
                                    if let DocumentKey::Ident(field_name) = key {
                                        let field_node = doc.get_node(*field_node_id);
                                        if let Some(field_schema) =
                                            Self::extract_field_schema_from_node(
                                                doc,
                                                field_name.as_ref(),
                                                field_node,
                                            )?
                                        {
                                            fields.insert(
                                                KeyCmpValue::String(field_name.to_string()),
                                                field_schema,
                                            );
                                        }
                                    }
                                }
                            }
                            schema.type_expr = Type::Array(Box::new(Type::Object(ObjectSchema {
                                fields,
                                additional_properties: None,
                            })));
                            has_schema = true;
                        }
                        _ => {}
                    }
                }
                "prefer" => {
                    // Handle $prefer extensions
                    if let NodeValue::Map { entries, .. } = &ext_node.content {
                        for (key, pref_node_id) in entries {
                            if let DocumentKey::Ident(pref_name) = key {
                                let pref_node = doc.get_node(*pref_node_id);
                                match pref_name.as_ref() {
                                    "section" => {
                                        if let NodeValue::Bool { value: b, .. } = &pref_node.content
                                        {
                                            schema.preferences.section = Some(*b);
                                            has_schema = true;
                                        }
                                    }
                                    "array" => {
                                        if let NodeValue::Bool { value: b, .. } = &pref_node.content
                                        {
                                            schema.preferences.array = Some(*b);
                                            has_schema = true;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                "rename" => {
                    // Handle $rename extension
                    if let NodeValue::String { value: rename, .. } = &ext_node.content {
                        schema.serde.rename = Some(rename.clone());
                        has_schema = true;
                    }
                }
                "rename-all" => {
                    // Handle $rename-all extension
                    if let NodeValue::String {
                        value: rename_all, ..
                    } = &ext_node.content
                    {
                        schema.serde.rename_all = Some(rename_all.parse()?);
                        has_schema = true;
                    }
                }
                _ => {}
            }
        }

        // Check if the value itself implies a schema
        match &node.content {
            NodeValue::Path { value: path, .. } => {
                // Field with direct type: field = .string
                if !has_schema && let Some(type_expr) = Type::from_path_segments(&path.0) {
                    schema.type_expr = type_expr;
                    has_schema = true;
                }
            }
            NodeValue::Map { .. } => {
                // Check if this is a map containing fields with schemas (i.e., an object schema)
                let mut fields = IndexMap::new();
                let mut has_schema_fields = false;

                if let NodeValue::Map { entries, .. } = &node.content {
                    for (key, field_node_id) in entries {
                        match key {
                            DocumentKey::Ident(field_name) => {
                                let field_node = doc.get_node(*field_node_id);
                                if let Some(field_schema) = Self::extract_field_schema_from_node(
                                    doc,
                                    field_name.as_ref(),
                                    field_node,
                                )? {
                                    fields.insert(
                                        KeyCmpValue::String(field_name.to_string()),
                                        field_schema,
                                    );
                                    has_schema_fields = true;
                                }
                            }
                            DocumentKey::Value(val) => {
                                // Handle quoted field names
                                if let KeyCmpValue::String(field_name) = val
                                    && let field_node = doc.get_node(*field_node_id)
                                    && let Some(field_schema) =
                                        Self::extract_field_schema_from_node(
                                            doc, field_name, field_node,
                                        )?
                                {
                                    fields.insert(
                                        KeyCmpValue::String(field_name.clone()),
                                        field_schema,
                                    );
                                    has_schema_fields = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                if has_schema_fields {
                    // This is an object with schema fields
                    match &mut schema.type_expr {
                        Type::Any => {
                            // No type set yet, create an object type
                            let obj_schema = ObjectSchema {
                                fields,
                                additional_properties: None,
                            };
                            schema.type_expr = Type::Object(obj_schema);
                        }
                        Type::Object(existing_obj) => {
                            // Type is already Object, merge the fields
                            for (key, field_schema) in fields {
                                existing_obj.fields.insert(key, field_schema);
                            }
                        }
                        _ => {
                            // Type is set to something else, can't add object fields
                            // This is a conflict but we'll keep the existing type
                        }
                    }
                    has_schema = true;
                }
            }
            _ if has_schema => {
                // This is a field with both schema and default value
                if let Some(val) = Self::node_content_to_serde_value(&node.content) {
                    schema.default_value = Some(val);
                }
            }
            _ => {}
        }

        if has_schema {
            Ok(Some(schema))
        } else {
            Ok(None)
        }
    }

    /// Extract all variants from a variants node
    fn extract_all_variants_from_node(
        &self,
        doc: &EureDocument,
        variants_node: &Node,
    ) -> Result<IndexMap<KeyCmpValue, ObjectSchema>, SchemaError> {
        let mut variants = IndexMap::new();

        if let NodeValue::Map { entries, .. } = &variants_node.content {
            for (key, node_id) in entries {
                if let DocumentKey::Ident(variant_name) = key {
                    let variant_node = doc.get_node(*node_id);
                    let variant_schema =
                        self.extract_variant_schema(doc, variant_name.as_ref(), variant_node)?;
                    variants.insert(
                        KeyCmpValue::String(variant_name.to_string()),
                        variant_schema,
                    );
                }
            }
        }

        Ok(variants)
    }

    /// Extract schema for a single variant
    fn extract_variant_schema(
        &self,
        doc: &EureDocument,
        _variant_name: &str,
        variant_node: &Node,
    ) -> Result<ObjectSchema, SchemaError> {
        let mut fields = IndexMap::new();

        if let NodeValue::Map { .. } = &variant_node.content {
            self.extract_variant_fields(doc, &mut fields, variant_node)?;
        }

        Ok(ObjectSchema {
            fields,
            additional_properties: None,
        })
    }

    /// Recursively extract variant fields
    fn extract_variant_fields(
        &self,
        doc: &EureDocument,
        fields: &mut IndexMap<KeyCmpValue, FieldSchema>,
        node: &Node,
    ) -> Result<(), SchemaError> {
        if let NodeValue::Map { entries, .. } = &node.content {
            for (key, node_id) in entries {
                match key {
                    DocumentKey::Ident(field_name) => {
                        let field_node = doc.get_node(*node_id);

                        // Check if this field has an array extension
                        if let Some(array_node_id) = field_node.extensions.get(&identifiers::ARRAY)
                        {
                            let array_node = doc.get_node(*array_node_id);
                            let array_field = self.extract_array_field(doc, array_node)?;
                            fields.insert(KeyCmpValue::String(field_name.to_string()), array_field);
                        } else if let Some(field_schema) = Self::extract_field_schema_from_node(
                            doc,
                            field_name.as_ref(),
                            field_node,
                        )? {
                            fields
                                .insert(KeyCmpValue::String(field_name.to_string()), field_schema);
                        }
                    }
                    DocumentKey::Value(val) => {
                        // Handle quoted field names in variants
                        if let KeyCmpValue::String(field_name) = val
                            && let field_node = doc.get_node(*node_id)
                            && let Some(field_schema) =
                                Self::extract_field_schema_from_node(doc, field_name, field_node)?
                        {
                            fields.insert(KeyCmpValue::String(field_name.clone()), field_schema);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Extract array field schema
    fn extract_array_field(
        &self,
        doc: &EureDocument,
        array_node: &Node,
    ) -> Result<FieldSchema, SchemaError> {
        match &array_node.content {
            NodeValue::Path { value: path, .. } => {
                // Simple array type: field.$array = .string
                let elem_type = Type::from_path_segments(&path.0)
                    .ok_or_else(|| SchemaError::InvalidTypePath(path_to_display_string(path)))?;
                Ok(FieldSchema {
                    type_expr: Type::Array(Box::new(elem_type)),
                    optional: false,
                    ..Default::default()
                })
            }
            NodeValue::Map { .. } => {
                // Complex array with object elements
                let mut elem_fields = IndexMap::new();
                self.extract_variant_fields(doc, &mut elem_fields, array_node)?;

                Ok(FieldSchema {
                    type_expr: Type::Array(Box::new(Type::Object(ObjectSchema {
                        fields: elem_fields,
                        additional_properties: None,
                    }))),
                    optional: false,
                    ..Default::default()
                })
            }
            _ => Err(SchemaError::InvalidField(
                "Invalid array definition".to_string(),
            )),
        }
    }

    /// Helper to extract usize from NodeContent
    fn extract_usize_from_node(content: &NodeValue) -> Option<usize> {
        match content {
            NodeValue::I64 { value, .. } => Some(*value as usize),
            NodeValue::U64 { value, .. } => Some(*value as usize),
            _ => None,
        }
    }

    /// Helper to extract f64 from NodeContent
    fn extract_f64_from_node(content: &NodeValue) -> Option<f64> {
        match content {
            NodeValue::I64 { value, .. } => Some(*value as f64),
            NodeValue::U64 { value, .. } => Some(*value as f64),
            NodeValue::F32 { value, .. } => Some(*value as f64),
            NodeValue::F64 { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Convert NodeContent to serde_json::Value for default values
    fn node_content_to_serde_value(content: &NodeValue) -> Option<serde_json::Value> {
        match content {
            NodeValue::Null { .. } => Some(serde_json::Value::Null),
            NodeValue::Bool { value, .. } => Some(serde_json::Value::Bool(*value)),
            NodeValue::I64 { value, .. } => Some(serde_json::Value::Number((*value).into())),
            NodeValue::U64 { value, .. } => Some(serde_json::Value::Number((*value).into())),
            NodeValue::F32 { value, .. } => {
                serde_json::Number::from_f64(*value as f64).map(serde_json::Value::Number)
            }
            NodeValue::F64 { value, .. } => {
                serde_json::Number::from_f64(*value).map(serde_json::Value::Number)
            }
            NodeValue::String { value, .. } => Some(serde_json::Value::String(value.clone())),
            NodeValue::Code { value, .. } => Some(serde_json::Value::String(value.content.clone())),
            NodeValue::CodeBlock { value, .. } => {
                Some(serde_json::Value::String(value.content.clone()))
            }
            NodeValue::NamedCode { value, .. } => {
                Some(serde_json::Value::String(value.content.clone()))
            }
            NodeValue::Path { value, .. } => {
                Some(serde_json::Value::String(path_to_display_string(value)))
            }
            NodeValue::Hole { .. } => None,
            // For complex types, we'll return None as they require recursive conversion
            NodeValue::Array { .. } | NodeValue::Map { .. } | NodeValue::Tuple { .. } => None,
        }
    }
}
