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

        rec.deny_unknown_fields()?;

        Ok(TextSchema {
            language,
            min_length,
            max_length,
            pattern,
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
}

impl ParseDocument<'_> for ParsedIntegerSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        let range = rec.field_optional::<String>("range")?;
        let multiple_of = rec.field_optional::<BigInt>("multiple-of")?;

        rec.deny_unknown_fields()?;

        Ok(ParsedIntegerSchema { range, multiple_of })
    }
}

/// Parsed float schema - syntactic representation with range as string.
#[derive(Debug, Clone)]
pub struct ParsedFloatSchema {
    /// Range constraint as string
    pub range: Option<String>,
    /// Multiple-of constraint
    pub multiple_of: Option<f64>,
}

impl ParseDocument<'_> for ParsedFloatSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        let range = rec.field_optional::<String>("range")?;
        let multiple_of = rec.field_optional::<f64>("multiple-of")?;

        rec.deny_unknown_fields()?;

        Ok(ParsedFloatSchema { range, multiple_of })
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
}

impl ParseDocument<'_> for ParsedArraySchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        let item = rec.field_node("item")?;
        let min_length = rec.field_optional::<u32>("min-length")?;
        let max_length = rec.field_optional::<u32>("max-length")?;
        let unique = rec.field_optional::<bool>("unique")?.unwrap_or(false);
        let contains = rec.field_node_optional("contains");

        rec.deny_unknown_fields()?;

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
}

impl ParseDocument<'_> for ParsedMapSchema {
    fn parse(doc: &EureDocument, node_id: NodeId) -> Result<Self, ParseError> {
        let mut rec = doc.parse_record(node_id)?;

        let key = rec.field_node("key")?;
        let value = rec.field_node("value")?;
        let min_size = rec.field_optional::<u32>("min-size")?;
        let max_size = rec.field_optional::<u32>("max-size")?;

        rec.deny_unknown_fields()?;

        Ok(ParsedMapSchema {
            key,
            value,
            min_size,
            max_size,
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
        // First try to parse as string literal
        if let Ok(value) = doc.parse::<&str>(node_id) {
            return match value {
                "deny" => Ok(ParsedUnknownFieldsPolicy::Deny),
                "allow" => Ok(ParsedUnknownFieldsPolicy::Allow),
                _ => Err(ParseError {
                    node_id,
                    kind: ParseErrorKind::UnknownVariant(value.to_string()),
                }),
            };
        }

        // Otherwise treat as schema NodeId
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

/// Parsed tuple schema with NodeId references.
#[derive(Debug, Clone)]
pub struct ParsedTupleSchema {
    /// Schema for each element by position (NodeId references)
    pub elements: Vec<NodeId>,
    /// Binding style for formatting
    pub binding_style: Option<BindingStyle>,
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

        rec.deny_unknown_fields()?;

        // Parse $ext-type.binding-style
        let mut ext = doc.parse_extension(node_id);
        let binding_style = ext.field_optional::<BindingStyle>("binding-style")?;
        ext.allow_unknown_fields();

        Ok(ParsedTupleSchema {
            elements,
            binding_style,
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

/// Parsed extension type schema with NodeId reference.
#[derive(Debug, Clone)]
pub struct ParsedExtTypeSchema {
    /// Schema for the extension value (NodeId reference)
    pub schema: NodeId,
    /// Whether the extension is optional (default: false = required)
    pub optional: bool,
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
