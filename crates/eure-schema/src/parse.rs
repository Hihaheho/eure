//! ParseDocument implementations for schema types.
//!
//! This module provides two categories of types:
//!
//! 1. **ParseDocument implementations for existing types** - Types that don't contain
//!    `SchemaNodeId` can implement `ParseDocument` directly (e.g., `BindingStyle`, `TextSchema`).
//!
//! 2. **Parsed types** - Syntactic representations of schema types that use `NodeId`
//!    instead of `SchemaNodeId` (e.g., `ParsedArraySchema`, `ParsedRecordSchema`).
//!
//! # Architecture
//!
//! ```text
//! EureDocument
//!     ↓ ParseDocument trait
//! ParsedSchemaNode, ParsedArraySchema, ...
//!     ↓ Converter (convert.rs)
//! SchemaDocument, SchemaNode, ArraySchema, ...
//! ```

use std::collections::HashMap;

use eure_value::data_model::VariantRepr;
use eure_value::document::{EureDocument, NodeId};
use eure_value::identifier::Identifier;
use eure_value::parse::{ParseDocument, ParseError, ParseErrorKind};
use num_bigint::BigInt;

use crate::{BindingStyle, Description, TextSchema, TypeReference};

// ============================================================================
// ParseDocument for existing types (no NodeId)
// ============================================================================

impl ParseDocument<'_> for BindingStyle {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let value: &str = doc.parse(node_id)?;
        match value {
            "auto" => Ok(BindingStyle::Auto),
            "passthrough" => Ok(BindingStyle::Passthrough),
            "section" => Ok(BindingStyle::Section),
            "nested" => Ok(BindingStyle::Nested),
            "binding" => Ok(BindingStyle::Binding),
            "section-binding" => Ok(BindingStyle::SectionBinding),
            "section-root-binding" => Ok(BindingStyle::SectionRootBinding),
            _ => Err(ParseError {
                node_id,
                kind: ParseErrorKind::UnknownVariant(value.to_string()),
            }),
        }
    }
}

impl ParseDocument<'_> for Description {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let node = doc.node(node_id);

        // Check for $variant extension
        let variant_ident: Identifier = "variant".parse().unwrap();
        if let Some(&variant_node_id) = node.extensions.get(&variant_ident) {
            let variant: &str = doc.parse(variant_node_id)?;
            match variant {
                "string" => {
                    let text: String = doc.parse(node_id)?;
                    Ok(Description::String(text))
                }
                "markdown" => {
                    let text: String = doc.parse(node_id)?;
                    Ok(Description::Markdown(text))
                }
                _ => Err(ParseError {
                    node_id,
                    kind: ParseErrorKind::UnknownVariant(variant.to_string()),
                }),
            }
        } else {
            // Default: treat as string (plain text)
            let text: String = doc.parse(node_id)?;
            Ok(Description::String(text))
        }
    }
}

impl ParseDocument<'_> for TypeReference {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        // TypeReference is parsed from a path like `$types.my-type` or `$types.namespace.type`
        // The path is stored as text in inline code format
        let path: &str = doc.parse(node_id)?;

        // Parse the path: should start with "$types." followed by name or namespace.name
        let path = path.strip_prefix("$types.").ok_or_else(|| ParseError {
            node_id,
            kind: ParseErrorKind::InvalidPattern {
                pattern: "$types.<name> or $types.<namespace>.<name>".to_string(),
                value: path.to_string(),
            },
        })?;

        // Split by '.' to get parts
        let parts: Vec<&str> = path.split('.').collect();
        match parts.as_slice() {
            [name] => {
                let name: Identifier = name.parse().map_err(|e| ParseError {
                    node_id,
                    kind: ParseErrorKind::InvalidIdentifier(e),
                })?;
                Ok(TypeReference {
                    namespace: None,
                    name,
                })
            }
            [namespace, name] => {
                let name: Identifier = name.parse().map_err(|e| ParseError {
                    node_id,
                    kind: ParseErrorKind::InvalidIdentifier(e),
                })?;
                Ok(TypeReference {
                    namespace: Some((*namespace).to_string()),
                    name,
                })
            }
            _ => Err(ParseError {
                node_id,
                kind: ParseErrorKind::InvalidPattern {
                    pattern: "$types.<name> or $types.<namespace>.<name>".to_string(),
                    value: format!("$types.{}", path),
                },
            }),
        }
    }
}

impl ParseDocument<'_> for TextSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        let language = rec.field_optional::<String>("language")?;
        let min_length = rec.field_optional::<u32>("min-length")?;
        let max_length = rec.field_optional::<u32>("max-length")?;
        let pattern = rec.field_optional::<String>("pattern")?;

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, node_id)| (name.to_string(), node_id))
            .collect();
        rec.allow_unknown_fields()?;

        Ok(TextSchema {
            language,
            min_length,
            max_length,
            pattern,
            unknown_fields,
        })
    }
}

// ============================================================================
// Parsed types (contain NodeId instead of SchemaNodeId)
// ============================================================================

/// Parsed integer schema - syntactic representation with range as string.
#[derive(Debug, Clone)]
pub struct ParsedIntegerSchema {
    /// Range constraint as string (e.g., "[0, 100)", "(-∞, 0]")
    pub range: Option<String>,
    /// Multiple-of constraint
    pub multiple_of: Option<BigInt>,
    /// Unknown fields (for future extensions like "flatten")
    pub unknown_fields: HashMap<String, NodeId>,
}

impl ParseDocument<'_> for ParsedIntegerSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        let range = rec.field_optional::<String>("range")?;
        let multiple_of = rec.field_optional::<BigInt>("multiple-of")?;

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, node_id)| (name.to_string(), node_id))
            .collect();
        rec.allow_unknown_fields()?;

        Ok(ParsedIntegerSchema {
            range,
            multiple_of,
            unknown_fields,
        })
    }
}

/// Parsed float schema - syntactic representation with range as string.
#[derive(Debug, Clone)]
pub struct ParsedFloatSchema {
    /// Range constraint as string
    pub range: Option<String>,
    /// Multiple-of constraint
    pub multiple_of: Option<f64>,
    /// Unknown fields (for future extensions like "flatten")
    pub unknown_fields: HashMap<String, NodeId>,
}

impl ParseDocument<'_> for ParsedFloatSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        let range = rec.field_optional::<String>("range")?;
        let multiple_of = rec.field_optional::<f64>("multiple-of")?;

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, node_id)| (name.to_string(), node_id))
            .collect();
        rec.allow_unknown_fields()?;

        Ok(ParsedFloatSchema {
            range,
            multiple_of,
            unknown_fields,
        })
    }
}

/// Parsed array schema with NodeId references.
#[derive(Debug, Clone)]
pub struct ParsedArraySchema {
    /// Schema for array elements
    pub item: NodeId,
    /// Minimum number of elements
    pub min_length: Option<u32>,
    /// Maximum number of elements
    pub max_length: Option<u32>,
    /// All elements must be unique
    pub unique: bool,
    /// Array must contain at least one element matching this schema
    pub contains: Option<NodeId>,
    /// Binding style for formatting
    pub binding_style: Option<BindingStyle>,
    /// Unknown fields (for future extensions like "flatten")
    pub unknown_fields: HashMap<String, NodeId>,
}

impl ParseDocument<'_> for ParsedArraySchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        let item = rec.field_node("item")?;
        let min_length = rec.field_optional::<u32>("min-length")?;
        let max_length = rec.field_optional::<u32>("max-length")?;
        let unique = rec.field_optional::<bool>("unique")?.unwrap_or(false);
        let contains = rec.field_node_optional("contains");

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, node_id)| (name.to_string(), node_id))
            .collect();
        rec.allow_unknown_fields()?;

        // Parse $ext-type.binding-style
        let mut ext = doc.parse_extension(node_id);
        let binding_style = ext.field_optional::<BindingStyle>("binding-style")?;
        ext.allow_unknown_fields();

        Ok(ParsedArraySchema {
            item,
            min_length,
            max_length,
            unique,
            contains,
            binding_style,
            unknown_fields,
        })
    }
}

/// Parsed map schema with NodeId references.
#[derive(Debug, Clone)]
pub struct ParsedMapSchema {
    /// Schema for keys
    pub key: NodeId,
    /// Schema for values
    pub value: NodeId,
    /// Minimum number of key-value pairs
    pub min_size: Option<u32>,
    /// Maximum number of key-value pairs
    pub max_size: Option<u32>,
    /// Unknown fields (for future extensions like "flatten")
    pub unknown_fields: HashMap<String, NodeId>,
}

impl ParseDocument<'_> for ParsedMapSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        let key = rec.field_node("key")?;
        let value = rec.field_node("value")?;
        let min_size = rec.field_optional::<u32>("min-size")?;
        let max_size = rec.field_optional::<u32>("max-size")?;

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, node_id)| (name.to_string(), node_id))
            .collect();
        rec.allow_unknown_fields()?;

        Ok(ParsedMapSchema {
            key,
            value,
            min_size,
            max_size,
            unknown_fields,
        })
    }
}

/// Parsed record field schema with NodeId reference.
#[derive(Debug, Clone)]
pub struct ParsedRecordFieldSchema {
    /// Schema for this field's value (NodeId reference)
    pub schema: NodeId,
    /// Field is optional (defaults to false = required)
    pub optional: bool,
    /// Binding style for this field
    pub binding_style: Option<BindingStyle>,
}

impl ParseDocument<'_> for ParsedRecordFieldSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut ext = doc.parse_extension(node_id);

        let optional = ext.field_optional::<bool>("optional")?.unwrap_or(false);
        let binding_style = ext.field_optional::<BindingStyle>("binding-style")?;

        // Allow other extensions (description, deprecated, etc.)
        ext.allow_unknown_fields();

        Ok(ParsedRecordFieldSchema {
            schema: node_id, // The node itself is the schema
            optional,
            binding_style,
        })
    }
}

/// Policy for handling fields not defined in record properties.
#[derive(Debug, Clone, Default)]
pub enum ParsedUnknownFieldsPolicy {
    /// Deny unknown fields (default, strict)
    #[default]
    Deny,
    /// Allow any unknown fields without validation
    Allow,
    /// Unknown fields must match this schema (NodeId reference)
    Schema(NodeId),
}

impl ParseDocument<'_> for ParsedUnknownFieldsPolicy {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let node = doc.node(node_id);

        // Check if it's a text value that could be a policy literal ("deny" or "allow")
        if let NodeValue::Primitive(PrimitiveValue::Text(text)) = &node.content {
            // Only treat plaintext (not inline code) as policy literals
            if text.language == Language::Plaintext {
                return match text.as_str() {
                    "deny" => Ok(ParsedUnknownFieldsPolicy::Deny),
                    "allow" => Ok(ParsedUnknownFieldsPolicy::Allow),
                    _ => Err(ParseError {
                        node_id,
                        kind: ParseErrorKind::UnknownVariant(text.as_str().to_string()),
                    }),
                };
            }
        }

        // Otherwise treat as schema NodeId (including inline code like `integer`)
        Ok(ParsedUnknownFieldsPolicy::Schema(node_id))
    }
}

/// Parsed record schema with NodeId references.
#[derive(Debug, Clone, Default)]
pub struct ParsedRecordSchema {
    /// Fixed field schemas (field name -> field schema with metadata)
    pub properties: HashMap<String, ParsedRecordFieldSchema>,
    /// Policy for unknown/additional fields
    pub unknown_fields: ParsedUnknownFieldsPolicy,
}

impl ParseDocument<'_> for ParsedRecordSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        // Parse $unknown-fields extension
        let mut ext = doc.parse_extension(node_id);
        let unknown_fields = ext
            .field_optional::<ParsedUnknownFieldsPolicy>("unknown-fields")?
            .unwrap_or_default();
        ext.allow_unknown_fields();

        // Parse all fields in the map as record properties
        let rec = doc.parse_record(node_id)?;
        let mut properties = HashMap::new();

        for (field_name, field_node_id) in rec.unknown_fields() {
            let field_schema = ParsedRecordFieldSchema::parse(doc, field_node_id)?;
            properties.insert(field_name.to_string(), field_schema);
        }

        Ok(ParsedRecordSchema {
            properties,
            unknown_fields,
        })
    }
}

/// Parsed tuple schema with NodeId references.
#[derive(Debug, Clone)]
pub struct ParsedTupleSchema {
    /// Schema for each element by position (NodeId references)
    pub elements: Vec<NodeId>,
    /// Binding style for formatting
    pub binding_style: Option<BindingStyle>,
    /// Unknown fields (for future extensions like "flatten")
    pub unknown_fields: HashMap<String, NodeId>,
}

impl ParseDocument<'_> for ParsedTupleSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        // elements is an array of NodeIds
        let elements_node = rec.field_node("elements")?;
        let elements: Vec<NodeId> = {
            let array = doc.parse::<&eure_value::document::node::NodeArray>(elements_node)?;
            array.iter().copied().collect()
        };

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, node_id)| (name.to_string(), node_id))
            .collect();
        rec.allow_unknown_fields()?;

        // Parse $ext-type.binding-style
        let mut ext = doc.parse_extension(node_id);
        let binding_style = ext.field_optional::<BindingStyle>("binding-style")?;
        ext.allow_unknown_fields();

        Ok(ParsedTupleSchema {
            elements,
            binding_style,
            unknown_fields,
        })
    }
}

/// Parsed union schema with NodeId references.
#[derive(Debug, Clone)]
pub struct ParsedUnionSchema {
    /// Variant definitions (variant name -> schema NodeId)
    pub variants: HashMap<String, NodeId>,
    /// Priority order for variant matching in untagged unions
    pub priority: Option<Vec<String>>,
    /// Variant representation strategy
    pub repr: VariantRepr,
}

impl ParseDocument<'_> for ParsedUnionSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;
        let mut variants = HashMap::new();

        // Check for variants = { ... } field
        if let Some(variants_node_id) = rec.field_node_optional("variants") {
            let variants_rec = doc.parse_record(variants_node_id)?;
            for (name, var_node_id) in variants_rec.unknown_fields() {
                variants.insert(name.to_string(), var_node_id);
            }
        }

        // Parse priority
        let priority = rec.field_optional::<Vec<String>>("priority")?;
        rec.allow_unknown_fields()?;

        // Parse $variant-repr extension
        let mut ext = doc.parse_extension(node_id);
        let repr = ext
            .field_optional::<VariantRepr>("variant-repr")?
            .unwrap_or_default();
        ext.allow_unknown_fields();

        Ok(ParsedUnionSchema {
            variants,
            priority,
            repr,
        })
    }
}

/// Parsed extension type schema with NodeId reference.
#[derive(Debug, Clone)]
pub struct ParsedExtTypeSchema {
    /// Schema for the extension value (NodeId reference)
    pub schema: NodeId,
    /// Whether the extension is optional (default: false = required)
    pub optional: bool,
}

impl ParseDocument<'_> for ParsedExtTypeSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut ext = doc.parse_extension(node_id);
        let optional = ext.field_optional::<bool>("optional")?.unwrap_or(false);
        ext.allow_unknown_fields();

        Ok(ParsedExtTypeSchema {
            schema: node_id,
            optional,
        })
    }
}

/// Parsed schema metadata - cascading metadata from $cascade-ext-types.
#[derive(Debug, Clone, Default)]
pub struct ParsedSchemaMetadata {
    /// Documentation/description
    pub description: Option<Description>,
    /// Marks as deprecated
    pub deprecated: bool,
    /// Default value (NodeId reference, not Value)
    pub default: Option<NodeId>,
    /// Example values in Eure code format
    pub examples: Option<Vec<String>>,
}

impl ParsedSchemaMetadata {
    /// Parse metadata from a node's extensions.
    pub fn parse_from_extensions(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut ext = doc.parse_extension(node_id);

        let description = ext.field_optional::<Description>("description")?;
        let deprecated = ext.field_optional::<bool>("deprecated")?.unwrap_or(false);
        let default = ext.field_node_optional("default");
        let examples = ext.field_optional::<Vec<String>>("examples")?;

        // Allow other extensions (codegen, etc.)
        ext.allow_unknown_fields();

        Ok(ParsedSchemaMetadata {
            description,
            deprecated,
            default,
            examples,
        })
    }
}

/// Parsed schema node content - the type definition with NodeId references.
#[derive(Debug, Clone)]
pub enum ParsedSchemaNodeContent {
    /// Any type - accepts any valid Eure value
    Any,
    /// Text type with constraints
    Text(TextSchema),
    /// Integer type with constraints
    Integer(ParsedIntegerSchema),
    /// Float type with constraints
    Float(ParsedFloatSchema),
    /// Boolean type (no constraints)
    Boolean,
    /// Null type
    Null,
    /// Literal type - accepts only the exact specified value (NodeId to the literal)
    Literal(NodeId),
    /// Array type with item schema
    Array(ParsedArraySchema),
    /// Map type with dynamic keys
    Map(ParsedMapSchema),
    /// Record type with fixed named fields
    Record(ParsedRecordSchema),
    /// Tuple type with fixed-length ordered elements
    Tuple(ParsedTupleSchema),
    /// Union type with named variants
    Union(ParsedUnionSchema),
    /// Type reference
    Reference(TypeReference),
}

/// Parsed schema node - full syntactic representation of a schema node.
#[derive(Debug, Clone)]
pub struct ParsedSchemaNode {
    /// The type definition content
    pub content: ParsedSchemaNodeContent,
    /// Cascading metadata
    pub metadata: ParsedSchemaMetadata,
    /// Extension type definitions for this node
    pub ext_types: HashMap<Identifier, ParsedExtTypeSchema>,
}

// ============================================================================
// Helper functions for parsing schema node content
// ============================================================================

use eure_value::document::node::NodeValue;
use eure_value::text::Language;
use eure_value::value::{PrimitiveValue, ValueKind};

/// Get the $variant extension value as a string if present.
fn get_variant_string(doc: &EureDocument, node_id: NodeId) -> Result<Option<String>, ParseError> {
    let mut ext = doc.parse_extension(node_id);
    let variant = ext.field_node_optional("variant");
    ext.allow_unknown_fields();

    match variant {
        Some(var_node_id) => {
            let node = doc.node(var_node_id);
            match &node.content {
                NodeValue::Primitive(PrimitiveValue::Text(t)) => Ok(Some(t.as_str().to_string())),
                NodeValue::Primitive(PrimitiveValue::Variant(v)) => Ok(Some(v.tag.clone())),
                _ => Err(ParseError {
                    node_id: var_node_id,
                    kind: ParseErrorKind::TypeMismatch {
                        expected: ValueKind::Text,
                        actual: node.content.value_kind().unwrap_or(ValueKind::Null),
                    },
                }),
            }
        }
        None => Ok(None),
    }
}

/// Parse a type reference string (e.g., "text", "integer", "$types.typename").
/// Returns ParsedSchemaNodeContent for the referenced type.
fn parse_type_reference_string(
    node_id: NodeId,
    s: &str,
) -> Result<ParsedSchemaNodeContent, ParseError> {
    if s.is_empty() {
        return Err(ParseError {
            node_id,
            kind: ParseErrorKind::InvalidPattern {
                pattern: "non-empty type reference".to_string(),
                value: String::new(),
            },
        });
    }

    let segments: Vec<&str> = s.split('.').collect();
    match segments.as_slice() {
        // Primitive types
        ["text"] => Ok(ParsedSchemaNodeContent::Text(TextSchema::default())),
        ["integer"] => Ok(ParsedSchemaNodeContent::Integer(ParsedIntegerSchema {
            range: None,
            multiple_of: None,
            unknown_fields: HashMap::new(),
        })),
        ["float"] => Ok(ParsedSchemaNodeContent::Float(ParsedFloatSchema {
            range: None,
            multiple_of: None,
            unknown_fields: HashMap::new(),
        })),
        ["boolean"] => Ok(ParsedSchemaNodeContent::Boolean),
        ["null"] => Ok(ParsedSchemaNodeContent::Null),
        ["any"] => Ok(ParsedSchemaNodeContent::Any),

        // Text with language: text.rust, text.email, etc.
        ["text", lang] => Ok(ParsedSchemaNodeContent::Text(TextSchema {
            language: Some((*lang).to_string()),
            ..Default::default()
        })),

        // Local type reference: $types.typename
        ["$types", type_name] => {
            let name: Identifier = type_name.parse().map_err(|e| ParseError {
                node_id,
                kind: ParseErrorKind::InvalidIdentifier(e),
            })?;
            Ok(ParsedSchemaNodeContent::Reference(TypeReference {
                namespace: None,
                name,
            }))
        }

        // External type reference: $types.namespace.typename
        ["$types", namespace, type_name] => {
            let name: Identifier = type_name.parse().map_err(|e| ParseError {
                node_id,
                kind: ParseErrorKind::InvalidIdentifier(e),
            })?;
            Ok(ParsedSchemaNodeContent::Reference(TypeReference {
                namespace: Some((*namespace).to_string()),
                name,
            }))
        }

        // Invalid pattern
        _ => Err(ParseError {
            node_id,
            kind: ParseErrorKind::InvalidPattern {
                pattern: "type reference (e.g., 'text', 'integer', '$types.name')".to_string(),
                value: s.to_string(),
            },
        }),
    }
}

/// Parse a primitive value as a schema node content.
fn parse_primitive_as_schema(
    _doc: &EureDocument,
    node_id: NodeId,
    prim: &PrimitiveValue,
) -> Result<ParsedSchemaNodeContent, ParseError> {
    match prim {
        PrimitiveValue::Text(t) => {
            match &t.language {
                // Inline code without language tag or eure-path: `text`, `$types.user`
                Language::Implicit => parse_type_reference_string(node_id, t.as_str()),
                Language::Other(lang) if lang == "eure-path" => {
                    parse_type_reference_string(node_id, t.as_str())
                }
                // Plaintext string "..." or other language - treat as literal
                _ => Ok(ParsedSchemaNodeContent::Literal(node_id)),
            }
        }
        // Other primitives are literals
        _ => Ok(ParsedSchemaNodeContent::Literal(node_id)),
    }
}

/// Parse a map node as a schema node content based on the variant.
fn parse_map_as_schema(
    doc: &EureDocument,
    node_id: NodeId,
    variant: Option<String>,
) -> Result<ParsedSchemaNodeContent, ParseError> {
    match variant.as_deref() {
        Some("text") => Ok(ParsedSchemaNodeContent::Text(doc.parse(node_id)?)),
        Some("integer") => Ok(ParsedSchemaNodeContent::Integer(doc.parse(node_id)?)),
        Some("float") => Ok(ParsedSchemaNodeContent::Float(doc.parse(node_id)?)),
        Some("boolean") => Ok(ParsedSchemaNodeContent::Boolean),
        Some("null") => Ok(ParsedSchemaNodeContent::Null),
        Some("any") => Ok(ParsedSchemaNodeContent::Any),
        Some("array") => Ok(ParsedSchemaNodeContent::Array(doc.parse(node_id)?)),
        Some("map") => Ok(ParsedSchemaNodeContent::Map(doc.parse(node_id)?)),
        Some("tuple") => Ok(ParsedSchemaNodeContent::Tuple(doc.parse(node_id)?)),
        Some("union") => Ok(ParsedSchemaNodeContent::Union(doc.parse(node_id)?)),
        Some("literal") => Ok(ParsedSchemaNodeContent::Literal(node_id)),
        Some("record") | None => Ok(ParsedSchemaNodeContent::Record(doc.parse(node_id)?)),
        Some(other) => Err(ParseError {
            node_id,
            kind: ParseErrorKind::UnknownVariant(other.to_string()),
        }),
    }
}

impl ParseDocument<'_> for ParsedSchemaNodeContent {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let node = doc.node(node_id);
        let variant = get_variant_string(doc, node_id)?;

        match &node.content {
            NodeValue::Uninitialized => Err(ParseError {
                node_id,
                kind: ParseErrorKind::UnexpectedUninitialized,
            }),

            NodeValue::Primitive(prim) => {
                // Check if this is explicitly a literal variant
                if variant.as_deref() == Some("literal") {
                    return Ok(ParsedSchemaNodeContent::Literal(node_id));
                }
                parse_primitive_as_schema(doc, node_id, prim)
            }

            NodeValue::Array(arr) => {
                // Array shorthand: [type] represents an array schema
                if arr.len() == 1 {
                    Ok(ParsedSchemaNodeContent::Array(ParsedArraySchema {
                        item: arr.0[0],
                        min_length: None,
                        max_length: None,
                        unique: false,
                        contains: None,
                        binding_style: None,
                        unknown_fields: HashMap::new(),
                    }))
                } else {
                    Err(ParseError {
                        node_id,
                        kind: ParseErrorKind::InvalidPattern {
                            pattern: "single-element array for array schema shorthand".to_string(),
                            value: format!("{}-element array", arr.len()),
                        },
                    })
                }
            }

            NodeValue::Tuple(tup) => {
                // Tuple shorthand: (type1, type2, ...) represents a tuple schema
                Ok(ParsedSchemaNodeContent::Tuple(ParsedTupleSchema {
                    elements: tup.0.clone(),
                    binding_style: None,
                    unknown_fields: HashMap::new(),
                }))
            }

            NodeValue::Map(_) => parse_map_as_schema(doc, node_id, variant),
        }
    }
}

impl ParseDocument<'_> for ParsedSchemaNode {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let content = doc.parse::<ParsedSchemaNodeContent>(node_id)?;
        let metadata = ParsedSchemaMetadata::parse_from_extensions(doc, node_id)?;
        let ext_types = parse_ext_types(doc, node_id)?;

        Ok(ParsedSchemaNode {
            content,
            metadata,
            ext_types,
        })
    }
}

/// Parse the $ext-type extension as a map of extension schemas.
fn parse_ext_types(
    doc: &EureDocument,
    node_id: NodeId,
) -> Result<HashMap<Identifier, ParsedExtTypeSchema>, ParseError> {
    let mut ext = doc.parse_extension(node_id);
    let ext_type_node = ext.field_node_optional("ext-type");
    ext.allow_unknown_fields();

    let mut result = HashMap::new();

    if let Some(ext_type_node_id) = ext_type_node {
        let rec = doc.parse_record(ext_type_node_id)?;
        // Collect all extension names first to avoid borrowing issues
        let ext_fields: Vec<_> = rec.unknown_fields().collect();

        for (name, type_node_id) in ext_fields {
            let ident: Identifier = name.parse().map_err(|e| ParseError {
                node_id: ext_type_node_id,
                kind: ParseErrorKind::InvalidIdentifier(e),
            })?;
            let schema = doc.parse::<ParsedExtTypeSchema>(type_node_id)?;
            result.insert(ident, schema);
        }

        // Allow unknown fields since we've processed all via unknown_fields() iterator
        // (unknown_fields() doesn't mark fields as accessed, so we can't use deny_unknown_fields)
        rec.allow_unknown_fields()?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eure_value::document::node::NodeValue;
    use eure_value::text::Text;
    use eure_value::value::PrimitiveValue;

    fn create_text_node(doc: &mut EureDocument, text: &str) -> NodeId {
        let root_id = doc.get_root_id();
        doc.node_mut(root_id).content =
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext(text.to_string())));
        root_id
    }

    #[test]
    fn test_binding_style_parse() {
        let mut doc = EureDocument::new();
        let node_id = create_text_node(&mut doc, "section");

        let result: BindingStyle = doc.parse(node_id).unwrap();
        assert_eq!(result, BindingStyle::Section);
    }

    #[test]
    fn test_binding_style_parse_unknown() {
        let mut doc = EureDocument::new();
        let node_id = create_text_node(&mut doc, "unknown");

        let result: Result<BindingStyle, _> = doc.parse(node_id);
        assert!(matches!(
            result.unwrap_err().kind,
            ParseErrorKind::UnknownVariant(_)
        ));
    }

    #[test]
    fn test_description_parse_default() {
        let mut doc = EureDocument::new();
        let node_id = create_text_node(&mut doc, "Hello world");

        let result: Description = doc.parse(node_id).unwrap();
        assert!(matches!(result, Description::String(s) if s == "Hello world"));
    }

    #[test]
    fn test_variant_repr_parse_string() {
        let mut doc = EureDocument::new();
        let node_id = create_text_node(&mut doc, "untagged");

        let result: VariantRepr = doc.parse(node_id).unwrap();
        assert_eq!(result, VariantRepr::Untagged);
    }
}
