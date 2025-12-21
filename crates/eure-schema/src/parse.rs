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

use std::collections::{BTreeMap, HashMap, HashSet};

use eure_document::data_model::VariantRepr;
use eure_document::document::NodeId;
use eure_document::identifier::Identifier;
use eure_document::parse::{
    DocumentParserExt as _, ParseContext, ParseDocument, ParseError, ParseErrorKind,
};
use num_bigint::BigInt;
use regex::Regex;

use crate::{BindingStyle, Description, TextSchema, TypeReference};

// ============================================================================
// ParseDocument for existing types (no NodeId)
// ============================================================================

impl ParseDocument<'_> for BindingStyle {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let value: &str = ctx.parse()?;
        match value {
            "auto" => Ok(BindingStyle::Auto),
            "passthrough" => Ok(BindingStyle::Passthrough),
            "section" => Ok(BindingStyle::Section),
            "nested" => Ok(BindingStyle::Nested),
            "binding" => Ok(BindingStyle::Binding),
            "section-binding" => Ok(BindingStyle::SectionBinding),
            "section-root-binding" => Ok(BindingStyle::SectionRootBinding),
            _ => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::UnknownVariant(value.to_string()),
            }),
        }
    }
}

impl ParseDocument<'_> for Description {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        use eure_document::data_model::VariantRepr;
        ctx.parse_union(VariantRepr::default())?
            .variant("string", String::parse.map(Description::String))
            .variant("markdown", String::parse.map(Description::Markdown))
            .parse()
    }
}

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
                pattern: "$types.<name> or $types.<namespace>.<name>".to_string(),
                value: path.to_string(),
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
                    pattern: "$types.<name> or $types.<namespace>.<name>".to_string(),
                    value: format!("$types.{}", path),
                },
            }),
        }
    }
}

impl ParseDocument<'_> for TextSchema {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        let language = rec.parse_field_optional::<String>("language")?;
        let min_length = rec.parse_field_optional::<u32>("min-length")?;
        let max_length = rec.parse_field_optional::<u32>("max-length")?;
        let pattern_str = rec.parse_field_optional::<String>("pattern")?;

        // Compile regex at parse time
        let pattern = pattern_str
            .map(|s| Regex::new(&s))
            .transpose()
            .map_err(|e| ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::InvalidPattern {
                    pattern: "valid regex".to_string(),
                    value: e.to_string(),
                },
            })?;

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, field_ctx)| (name.to_string(), field_ctx.node_id()))
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

impl ParseDocument<'_> for crate::SchemaRef {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut ext = ctx.parse_extension();
        let schema_ctx = ext.ext("schema")?;
        ext.allow_unknown_extensions();

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
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        let range = rec.parse_field_optional::<String>("range")?;
        let multiple_of = rec.parse_field_optional::<BigInt>("multiple-of")?;

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, ctx)| (name.to_string(), ctx.node_id()))
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
    /// Precision constraint ("f32" or "f64")
    pub precision: Option<String>,
    /// Unknown fields (for future extensions like "flatten")
    pub unknown_fields: HashMap<String, NodeId>,
}

impl ParseDocument<'_> for ParsedFloatSchema {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        let range = rec.parse_field_optional::<String>("range")?;
        let multiple_of = rec.parse_field_optional::<f64>("multiple-of")?;
        let precision = rec.parse_field_optional::<String>("precision")?;

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, ctx)| (name.to_string(), ctx.node_id()))
            .collect();
        rec.allow_unknown_fields()?;

        Ok(ParsedFloatSchema {
            range,
            multiple_of,
            precision,
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
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        let item = rec.field("item")?.node_id();
        let min_length = rec.parse_field_optional::<u32>("min-length")?;
        let max_length = rec.parse_field_optional::<u32>("max-length")?;
        let unique = rec.parse_field_optional::<bool>("unique")?.unwrap_or(false);
        let contains = rec.field_optional("contains").map(|ctx| ctx.node_id());

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, ctx)| (name.to_string(), ctx.node_id()))
            .collect();
        rec.allow_unknown_fields()?;

        // Parse $ext-type.binding-style
        let mut ext = ctx.parse_extension();
        let binding_style = ext.parse_ext_optional::<BindingStyle>("binding-style")?;
        ext.allow_unknown_extensions();

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
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        let key = rec.field("key")?.node_id();
        let value = rec.field("value")?.node_id();
        let min_size = rec.parse_field_optional::<u32>("min-size")?;
        let max_size = rec.parse_field_optional::<u32>("max-size")?;

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, ctx)| (name.to_string(), ctx.node_id()))
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
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut ext = ctx.parse_extension();

        let optional = ext.parse_ext_optional::<bool>("optional")?.unwrap_or(false);
        let binding_style = ext.parse_ext_optional::<BindingStyle>("binding-style")?;

        // Allow other extensions (description, deprecated, etc.)
        ext.allow_unknown_extensions();

        Ok(ParsedRecordFieldSchema {
            schema: ctx.node_id(), // The node itself is the schema
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
    pub properties: HashMap<String, ParsedRecordFieldSchema>,
    /// Policy for unknown/additional fields
    pub unknown_fields: ParsedUnknownFieldsPolicy,
}

impl ParseDocument<'_> for ParsedRecordSchema {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        // Parse $unknown-fields extension
        let mut ext = ctx.parse_extension();
        let unknown_fields = ext
            .parse_ext_optional::<ParsedUnknownFieldsPolicy>("unknown-fields")?
            .unwrap_or_default();
        ext.allow_unknown_extensions();

        // Parse all fields in the map as record properties
        let rec = ctx.parse_record()?;
        let mut properties = HashMap::new();

        for (field_name, field_ctx) in rec.unknown_fields() {
            let field_schema = ParsedRecordFieldSchema::parse(&field_ctx)?;
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
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;

        // elements is an array of NodeIds
        let elements_ctx = rec.field("elements")?;
        let elements: Vec<NodeId> = {
            let array = elements_ctx.parse::<&eure_document::document::node::NodeArray>()?;
            array.iter().copied().collect()
        };

        // Collect unknown fields for future extensions
        let unknown_fields: HashMap<String, NodeId> = rec
            .unknown_fields()
            .map(|(name, ctx)| (name.to_string(), ctx.node_id()))
            .collect();
        rec.allow_unknown_fields()?;

        // Parse $ext-type.binding-style
        let mut ext = ctx.parse_extension();
        let binding_style = ext.parse_ext_optional::<BindingStyle>("binding-style")?;
        ext.allow_unknown_extensions();

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
    pub variants: BTreeMap<String, NodeId>,
    /// Variants that use unambiguous semantics (try all, detect conflicts).
    /// All other variants use short-circuit semantics (first match wins).
    pub unambiguous: HashSet<String>,
    /// Variant representation strategy
    pub repr: VariantRepr,
    /// Variants that deny untagged matching (require explicit $variant)
    pub deny_untagged: HashSet<String>,
}

impl ParseDocument<'_> for ParsedUnionSchema {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut rec = ctx.parse_record()?;
        let mut variants = BTreeMap::new();
        let mut unambiguous = HashSet::new();
        let mut deny_untagged = HashSet::new();

        // Check for variants = { ... } field
        if let Some(variants_ctx) = rec.field_optional("variants") {
            let variants_rec = variants_ctx.parse_record()?;
            for (name, var_ctx) in variants_rec.unknown_fields() {
                variants.insert(name.to_string(), var_ctx.node_id());

                // Parse extensions on the variant value
                let mut ext = var_ctx.parse_extension();
                if ext
                    .parse_ext_optional::<bool>("deny-untagged")?
                    .unwrap_or(false)
                {
                    deny_untagged.insert(name.to_string());
                }
                if ext
                    .parse_ext_optional::<bool>("unambiguous")?
                    .unwrap_or(false)
                {
                    unambiguous.insert(name.to_string());
                }
                // There may other extensions to be parsed on the later stage. like $variant, $ext-type, etc.
                ext.allow_unknown_extensions();
            }
        }

        rec.allow_unknown_fields()?;

        // Parse $variant-repr extension
        let mut ext = ctx.parse_extension();
        let repr = ext
            .parse_ext_optional::<VariantRepr>("variant-repr")?
            .unwrap_or_default();
        ext.allow_unknown_extensions();

        Ok(ParsedUnionSchema {
            variants,
            unambiguous,
            repr,
            deny_untagged,
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
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let mut ext = ctx.parse_extension();
        let optional = ext.parse_ext_optional::<bool>("optional")?.unwrap_or(false);
        ext.allow_unknown_extensions();

        Ok(ParsedExtTypeSchema {
            schema: ctx.node_id(),
            optional,
        })
    }
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
        let mut ext = ctx.parse_extension();

        let description = ext.parse_ext_optional::<Description>("description")?;
        let deprecated = ext
            .parse_ext_optional::<bool>("deprecated")?
            .unwrap_or(false);
        let default = ext.ext_optional("default").map(|ctx| ctx.node_id());
        let examples = ext.parse_ext_optional::<Vec<NodeId>>("examples")?;

        // Allow other extensions (codegen, etc.)
        ext.allow_unknown_extensions();

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

use eure_document::document::node::NodeValue;
use eure_document::text::Language;
use eure_document::value::{PrimitiveValue, ValueKind};

/// Get the $variant extension value as a string if present.
fn get_variant_string(ctx: &ParseContext<'_>) -> Result<Option<String>, ParseError> {
    let mut ext = ctx.parse_extension();
    let variant_ctx = ext.ext_optional("variant");
    ext.allow_unknown_extensions();

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
            precision: None,
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

            NodeValue::Map(_) => parse_map_as_schema(ctx, variant),
        }
    }
}

impl ParseDocument<'_> for ParsedSchemaNode {
    type Error = ParseError;
    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let content = ctx.parse::<ParsedSchemaNodeContent>()?;
        let metadata = ParsedSchemaMetadata::parse_from_extensions(ctx)?;
        let ext_types = parse_ext_types(ctx)?;

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
) -> Result<HashMap<Identifier, ParsedExtTypeSchema>, ParseError> {
    let mut ext = ctx.parse_extension();
    let ext_type_ctx = ext.ext_optional("ext-type");
    ext.allow_unknown_extensions();

    let mut result = HashMap::new();

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
