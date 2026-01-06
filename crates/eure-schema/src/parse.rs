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

use eure_document::data_model::VariantRepr;
use eure_document::document::NodeId;
use eure_document::identifier::Identifier;
use eure_document::parse::{ParseContext, ParseDocument, ParseError, ParseErrorKind};
use indexmap::{IndexMap, IndexSet};
use num_bigint::BigInt;

use crate::{BindingStyle, Description, TextSchema, TypeReference};

impl ParseDocument<'_> for TypeReference {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        // TypeReference is parsed from a path like `$types.my-type` or `$types.namespace.type`
        // The path is stored as text in inline code format
        let path: &str = ctx.parse()?;

        // Parse the path: should start with "$types." followed by name or namespace.name
        let path = path.strip_prefix("$types.").ok_or_else(|| ParseError {
            node_id: ctx.node_id(),
            kind: ParseErrorKind::InvalidPattern {
                kind: "type reference".to_string(),
                reason: format!(
                    "expected '$types.<name>' or '$types.<namespace>.<name>', got '{}'",
                    path
                ),
            },
        })?;

        // Split by '.' to get parts
        let parts: Vec<&str> = path.split('.').collect();
        match parts.as_slice() {
            [name] => {
                let name: Identifier = name.parse().map_err(|e| ParseError {
                    node_id: ctx.node_id(),
                    kind: ParseErrorKind::InvalidIdentifier(e),
                })?;
                Ok(TypeReference {
                    namespace: None,
                    name,
                })
            }
            [namespace, name] => {
                let name: Identifier = name.parse().map_err(|e| ParseError {
                    node_id: ctx.node_id(),
                    kind: ParseErrorKind::InvalidIdentifier(e),
                })?;
                Ok(TypeReference {
                    namespace: Some((*namespace).to_string()),
                    name,
                })
            }
            _ => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::InvalidPattern {
                    kind: "type reference".to_string(),
                    reason: format!(
                        "expected '$types.<name>' or '$types.<namespace>.<name>', got '$types.{}'",
                        path
                    ),
                },
            }),
        }
    }
}

impl ParseDocument<'_> for crate::SchemaRef {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let schema_ctx = ctx.ext("schema")?;

        let path: String = schema_ctx.parse()?;
        Ok(crate::SchemaRef {
            path,
            node_id: schema_ctx.node_id(),
        })
    }
}

// ============================================================================
// Parsed types (contain NodeId instead of SchemaNodeId)
// ============================================================================

/// Parsed integer schema - syntactic representation with range as string.
#[derive(Debug, Clone, eure_macros::ParseDocument)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct ParsedIntegerSchema {
    /// Range constraint as string (e.g., "[0, 100)", "(-∞, 0]")
    #[eure(default)]
    pub range: Option<String>,
    /// Multiple-of constraint
    #[eure(default)]
    pub multiple_of: Option<BigInt>,
}

/// Parsed float schema - syntactic representation with range as string.
#[derive(Debug, Clone, eure_macros::ParseDocument)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct ParsedFloatSchema {
    /// Range constraint as string
    #[eure(default)]
    pub range: Option<String>,
    /// Multiple-of constraint
    #[eure(default)]
    pub multiple_of: Option<f64>,
    /// Precision constraint ("f32" or "f64")
    #[eure(default)]
    pub precision: Option<String>,
}

/// Parsed array schema with NodeId references.
#[derive(Debug, Clone, eure_macros::ParseDocument)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct ParsedArraySchema {
    /// Schema for array elements
    pub item: NodeId,
    /// Minimum number of elements
    #[eure(default)]
    pub min_length: Option<u32>,
    /// Maximum number of elements
    #[eure(default)]
    pub max_length: Option<u32>,
    /// All elements must be unique
    #[eure(default)]
    pub unique: bool,
    /// Array must contain at least one element matching this schema
    #[eure(default)]
    pub contains: Option<NodeId>,
    /// Binding style for formatting
    #[eure(ext, default)]
    pub binding_style: Option<BindingStyle>,
}

/// Parsed map schema with NodeId references.
#[derive(Debug, Clone, eure_macros::ParseDocument)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct ParsedMapSchema {
    /// Schema for keys
    pub key: NodeId,
    /// Schema for values
    pub value: NodeId,
    /// Minimum number of key-value pairs
    #[eure(default)]
    pub min_size: Option<u32>,
    /// Maximum number of key-value pairs
    #[eure(default)]
    pub max_size: Option<u32>,
}

/// Parsed record field schema with NodeId reference.
#[derive(Debug, Clone, eure_macros::ParseDocument)]
#[eure(crate = eure_document, parse_ext, rename_all = "kebab-case")]
pub struct ParsedRecordFieldSchema {
    /// Schema for this field's value (NodeId reference)
    #[eure(flatten_ext)]
    pub schema: NodeId,
    /// Field is optional (defaults to false = required)
    #[eure(default)]
    pub optional: bool,
    /// Binding style for this field
    #[eure(default)]
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
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let node = ctx.node();
        let node_id = ctx.node_id();

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
    pub properties: IndexMap<String, ParsedRecordFieldSchema>,
    /// Schemas to be flattened into this record
    pub flatten: Vec<NodeId>,
    /// Policy for unknown/additional fields
    pub unknown_fields: ParsedUnknownFieldsPolicy,
}

impl ParseDocument<'_> for ParsedRecordSchema {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        // Parse $unknown-fields extension
        let unknown_fields = ctx
            .parse_ext_optional::<ParsedUnknownFieldsPolicy>("unknown-fields")?
            .unwrap_or_default();

        // Parse $flatten extension - list of schemas to flatten into this record
        let flatten = ctx
            .parse_ext_optional::<Vec<NodeId>>("flatten")?
            .unwrap_or_default();

        // Parse all fields in the map as record properties
        let rec = ctx.parse_record()?;
        let mut properties = IndexMap::new();

        for (field_name, field_ctx) in rec.unknown_fields() {
            let field_schema = ParsedRecordFieldSchema::parse(&field_ctx)?;
            properties.insert(field_name.to_string(), field_schema);
        }

        Ok(ParsedRecordSchema {
            properties,
            flatten,
            unknown_fields,
        })
    }
}

/// Parsed tuple schema with NodeId references.
#[derive(Debug, Clone, eure_macros::ParseDocument)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct ParsedTupleSchema {
    /// Schema for each element by position (NodeId references)
    pub elements: Vec<NodeId>,
    /// Binding style for formatting
    #[eure(ext, default)]
    pub binding_style: Option<BindingStyle>,
}

/// Parsed union schema with NodeId references.
#[derive(Debug, Clone)]
pub struct ParsedUnionSchema {
    /// Variant definitions (variant name -> schema NodeId)
    pub variants: IndexMap<String, NodeId>,
    /// Variants that use unambiguous semantics (try all, detect conflicts).
    /// All other variants use short-circuit semantics (first match wins).
    pub unambiguous: IndexSet<String>,
    /// Variant representation strategy
    pub repr: VariantRepr,
    /// Variants that deny untagged matching (require explicit $variant)
    pub deny_untagged: IndexSet<String>,
}

impl ParseDocument<'_> for ParsedUnionSchema {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let rec = ctx.parse_record()?;
        let mut variants = IndexMap::new();
        let mut unambiguous = IndexSet::new();
        let mut deny_untagged = IndexSet::new();

        // Check for variants = { ... } field
        if let Some(variants_ctx) = rec.field_optional("variants") {
            let variants_rec = variants_ctx.parse_record()?;
            for (name, var_ctx) in variants_rec.unknown_fields() {
                variants.insert(name.to_string(), var_ctx.node_id());

                // Parse extensions on the variant value
                if var_ctx
                    .parse_ext_optional::<bool>("deny-untagged")?
                    .unwrap_or(false)
                {
                    deny_untagged.insert(name.to_string());
                }
                if var_ctx
                    .parse_ext_optional::<bool>("unambiguous")?
                    .unwrap_or(false)
                {
                    unambiguous.insert(name.to_string());
                }
            }
        }

        rec.allow_unknown_fields()?;

        // Parse $variant-repr extension
        let repr = ctx
            .parse_ext_optional::<VariantRepr>("variant-repr")?
            .unwrap_or_default();

        Ok(ParsedUnionSchema {
            variants,
            unambiguous,
            repr,
            deny_untagged,
        })
    }
}

/// Parsed extension type schema with NodeId reference.
#[derive(Debug, Clone, eure_macros::ParseDocument)]
#[eure(crate = eure_document, parse_ext)]
pub struct ParsedExtTypeSchema {
    /// Schema for the extension value (NodeId reference)
    #[eure(flatten_ext)]
    pub schema: NodeId,
    /// Whether the extension is optional (default: false = required)
    #[eure(default)]
    pub optional: bool,
}

/// Parsed schema metadata - extension metadata via $ext-type on $types.type.
#[derive(Debug, Clone, Default)]
pub struct ParsedSchemaMetadata {
    /// Documentation/description
    pub description: Option<Description>,
    /// Marks as deprecated
    pub deprecated: bool,
    /// Default value (NodeId reference, not Value)
    pub default: Option<NodeId>,
    /// Example values as NodeId references
    pub examples: Option<Vec<NodeId>>,
}

impl ParsedSchemaMetadata {
    /// Parse metadata from a node's extensions.
    pub fn parse_from_extensions(ctx: &ParseContext<'_>) -> Result<Self, ParseError> {
        let description = ctx.parse_ext_optional::<Description>("description")?;
        let deprecated = ctx
            .parse_ext_optional::<bool>("deprecated")?
            .unwrap_or(false);
        let default = ctx.ext_optional("default").map(|ctx| ctx.node_id());
        let examples = ctx.parse_ext_optional::<Vec<NodeId>>("examples")?;

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
    pub ext_types: IndexMap<Identifier, ParsedExtTypeSchema>,
}

// ============================================================================
// Helper functions for parsing schema node content
// ============================================================================

use eure_document::document::node::NodeValue;
use eure_document::text::Language;
use eure_document::value::{PrimitiveValue, ValueKind};

/// Get the $variant extension value as a string if present.
fn get_variant_string(ctx: &ParseContext<'_>) -> Result<Option<String>, ParseError> {
    let variant_ctx = ctx.ext_optional("variant");

    match variant_ctx {
        Some(var_ctx) => {
            let node = var_ctx.node();
            match &node.content {
                NodeValue::Primitive(PrimitiveValue::Text(t)) => Ok(Some(t.as_str().to_string())),
                _ => Err(ParseError {
                    node_id: var_ctx.node_id(),
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
                kind: "type reference".to_string(),
                reason: "expected non-empty type reference, got empty string".to_string(),
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
        })),
        ["float"] => Ok(ParsedSchemaNodeContent::Float(ParsedFloatSchema {
            range: None,
            multiple_of: None,
            precision: None,
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
                kind: "type reference".to_string(),
                reason: format!(
                    "expected 'text', 'integer', '$types.name', etc., got '{}'",
                    s
                ),
            },
        }),
    }
}

/// Parse a primitive value as a schema node content.
fn parse_primitive_as_schema(
    ctx: &ParseContext<'_>,
    prim: &PrimitiveValue,
) -> Result<ParsedSchemaNodeContent, ParseError> {
    let node_id = ctx.node_id();
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
    ctx: &ParseContext<'_>,
    variant: Option<String>,
) -> Result<ParsedSchemaNodeContent, ParseError> {
    let node_id = ctx.node_id();
    match variant.as_deref() {
        Some("text") => Ok(ParsedSchemaNodeContent::Text(ctx.parse()?)),
        Some("integer") => Ok(ParsedSchemaNodeContent::Integer(ctx.parse()?)),
        Some("float") => Ok(ParsedSchemaNodeContent::Float(ctx.parse()?)),
        Some("boolean") => Ok(ParsedSchemaNodeContent::Boolean),
        Some("null") => Ok(ParsedSchemaNodeContent::Null),
        Some("any") => Ok(ParsedSchemaNodeContent::Any),
        Some("array") => Ok(ParsedSchemaNodeContent::Array(ctx.parse()?)),
        Some("map") => Ok(ParsedSchemaNodeContent::Map(ctx.parse()?)),
        Some("tuple") => Ok(ParsedSchemaNodeContent::Tuple(ctx.parse()?)),
        Some("union") => Ok(ParsedSchemaNodeContent::Union(ctx.parse()?)),
        Some("literal") => Ok(ParsedSchemaNodeContent::Literal(node_id)),
        Some("record") | None => Ok(ParsedSchemaNodeContent::Record(ctx.parse()?)),
        Some(other) => Err(ParseError {
            node_id,
            kind: ParseErrorKind::UnknownVariant(other.to_string()),
        }),
    }
}

impl ParseDocument<'_> for ParsedSchemaNodeContent {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let node_id = ctx.node_id();
        let node = ctx.node();
        let variant = get_variant_string(ctx)?;

        match &node.content {
            NodeValue::Hole(_) => Err(ParseError {
                node_id,
                kind: ParseErrorKind::UnexpectedHole,
            }),

            NodeValue::Primitive(prim) => {
                // Check if this is explicitly a literal variant
                if variant.as_deref() == Some("literal") {
                    return Ok(ParsedSchemaNodeContent::Literal(node_id));
                }
                parse_primitive_as_schema(ctx, prim)
            }

            NodeValue::Array(arr) => {
                // Array shorthand: [type] represents an array schema
                if arr.len() == 1 {
                    Ok(ParsedSchemaNodeContent::Array(ParsedArraySchema {
                        item: arr.get(0).unwrap(),
                        min_length: None,
                        max_length: None,
                        unique: false,
                        contains: None,
                        binding_style: None,
                    }))
                } else {
                    Err(ParseError {
                        node_id,
                        kind: ParseErrorKind::InvalidPattern {
                            kind: "array schema shorthand".to_string(),
                            reason: format!(
                                "expected single-element array [type], got {}-element array",
                                arr.len()
                            ),
                        },
                    })
                }
            }

            NodeValue::Tuple(tup) => {
                // Tuple shorthand: (type1, type2, ...) represents a tuple schema
                Ok(ParsedSchemaNodeContent::Tuple(ParsedTupleSchema {
                    elements: tup.to_vec(),
                    binding_style: None,
                }))
            }

            NodeValue::Map(_) => parse_map_as_schema(ctx, variant),
        }
    }
}

impl ParseDocument<'_> for ParsedSchemaNode {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        // Create a flattened context so child parsers' deny_unknown_* are no-ops.
        // All accesses are recorded in the shared accessed set (via Rc).
        let flatten_ctx = ctx.flatten();

        // Parse schema-level extensions - marks $ext-type, $description, etc. as accessed
        let ext_types = parse_ext_types(&flatten_ctx)?;
        let metadata = ParsedSchemaMetadata::parse_from_extensions(&flatten_ctx)?;

        // Content parsing uses the flattened context
        let content = flatten_ctx.parse::<ParsedSchemaNodeContent>()?;

        // Note: We do NOT validate unknown extensions here because:
        // 1. At the document root, $types extension is handled by the converter
        // 2. Content types use flatten context, so their deny is already no-op
        // The caller (e.g., Converter) should handle document-level validation if needed.

        Ok(ParsedSchemaNode {
            content,
            metadata,
            ext_types,
        })
    }
}

/// Parse the $ext-type extension as a map of extension schemas.
fn parse_ext_types(
    ctx: &ParseContext<'_>,
) -> Result<IndexMap<Identifier, ParsedExtTypeSchema>, ParseError> {
    let ext_type_ctx = ctx.ext_optional("ext-type");

    let mut result = IndexMap::new();

    if let Some(ext_type_ctx) = ext_type_ctx {
        let rec = ext_type_ctx.parse_record()?;
        // Collect all extension names first to avoid borrowing issues
        let ext_fields: Vec<_> = rec.unknown_fields().collect();

        for (name, type_ctx) in ext_fields {
            let ident: Identifier = name.parse().map_err(|e| ParseError {
                node_id: ext_type_ctx.node_id(),
                kind: ParseErrorKind::InvalidIdentifier(e),
            })?;
            let schema = type_ctx.parse::<ParsedExtTypeSchema>()?;
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
    use eure_document::document::EureDocument;
    use eure_document::document::node::NodeValue;
    use eure_document::text::Text;
    use eure_document::value::PrimitiveValue;

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
        let err = result.unwrap_err();
        assert_eq!(
            err.kind,
            ParseErrorKind::UnknownVariant("unknown".to_string())
        );
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
