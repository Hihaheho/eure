//! Eure Schema types and structures
//!
//! This library provides schema type definitions for Eure documents,
//! following the specification in `assets/eure-schema.schema.eure`.
//!
//! # Type Variants
//!
//! All types are variants of `SchemaNodeContent`:
//!
//! **Primitives:**
//! - `Text` - Text type with optional language and length/pattern constraints
//! - `Integer` - Integer type with optional range and multiple-of constraints
//! - `Float` - Float type with optional range and multiple-of constraints
//! - `Boolean` - Boolean type (no constraints)
//! - `Null` - Null type
//! - `Any` - Any type (accepts any value)
//!
//! **Literal:**
//! - `Literal` - Exact value match (e.g., `status = "active"`)
//!
//! **Compounds:**
//! - `Record` - Fixed named fields
//! - `Array` - Ordered list with item type
//! - `Map` - Dynamic key-value pairs
//! - `Tuple` - Fixed-length ordered elements
//! - `Union` - Tagged union with named variants
//!
//! **Reference:**
//! - `Reference` - Type reference (local or cross-schema)

pub mod convert;
pub mod identifiers;
pub mod parse;
pub mod synth;
pub mod validate;

use eure_document::data_model::VariantRepr;
use eure_document::document::EureDocument;
use eure_document::identifier::Identifier;
use num_bigint::BigInt;
use regex::Regex;
use std::collections::HashMap;

// ============================================================================
// Schema Document
// ============================================================================

/// Schema document with arena-based node storage
#[derive(Debug, Clone)]
pub struct SchemaDocument {
    /// All schema nodes stored in a flat vector
    pub nodes: Vec<SchemaNode>,
    /// Root node reference
    pub root: SchemaNodeId,
    /// Named type definitions ($types)
    pub types: HashMap<Identifier, SchemaNodeId>,
}

/// Extension type definition with optionality
#[derive(Debug, Clone)]
pub struct ExtTypeSchema {
    /// Schema for the extension value
    pub schema: SchemaNodeId,
    /// Whether the extension is optional (default: false = required)
    pub optional: bool,
}

/// Reference to a schema node by index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchemaNodeId(pub usize);

/// A single schema node
#[derive(Debug, Clone)]
pub struct SchemaNode {
    /// The type definition, structure, and constraints
    pub content: SchemaNodeContent,
    /// Cascading metadata (description, deprecated, default, examples)
    pub metadata: SchemaMetadata,
    /// Extension type definitions for this node ($ext-type.X)
    pub ext_types: HashMap<Identifier, ExtTypeSchema>,
}

// ============================================================================
// Schema Node Content
// ============================================================================

/// Type definitions with their specific constraints
///
/// See spec: `eure-schema.schema.eure` lines 298-525
#[derive(Debug, Clone)]
pub enum SchemaNodeContent {
    // --- Primitives ---
    /// Any type - accepts any valid Eure value
    /// Spec: line 391
    Any,

    /// Text type
    ///
    /// # Language Matching
    ///
    /// When validating text values:
    /// - `Language::Plaintext` (from `"..."`) must match `.text` schema only
    /// - `Language::Implicit` (from `` `...` ``) can be coerced to any language by schema
    /// - `Language::Other(lang)` (from `` lang`...` ``) must match `.text.{lang}` schema
    ///
    /// Spec: lines 333-349
    Text(TextSchema),

    /// Integer type with optional constraints
    /// Spec: lines 360-364
    Integer(IntegerSchema),

    /// Float type with optional constraints
    /// Spec: lines 371-375
    Float(FloatSchema),

    /// Boolean type (no constraints)
    /// Spec: line 383
    Boolean,

    /// Null type
    /// Spec: line 387
    Null,

    // --- Literal ---
    /// Literal type - accepts only the exact specified value
    /// Spec: line 396
    Literal(EureDocument),

    // --- Compounds ---
    /// Array type with item schema and optional constraints
    /// Spec: lines 426-439
    Array(ArraySchema),

    /// Map type with dynamic keys
    /// Spec: lines 453-459
    Map(MapSchema),

    /// Record type with fixed named fields
    /// Spec: lines 401-410
    Record(RecordSchema),

    /// Tuple type with fixed-length ordered elements
    /// Spec: lines 465-468
    Tuple(TupleSchema),

    /// Union type with named variants
    /// Spec: lines 415-423
    Union(UnionSchema),

    // --- Reference ---
    /// Type reference (local or cross-schema)
    /// Spec: lines 506-510
    Reference(TypeReference),
}

// ============================================================================
// Primitive Type Schemas
// ============================================================================

/// Boundary condition for numeric constraints
///
/// Uses ADT to prevent invalid states (e.g., both inclusive and exclusive)
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Bound<T> {
    /// No constraint (-∞ or +∞)
    #[default]
    Unbounded,
    /// Inclusive bound (≤ or ≥)
    Inclusive(T),
    /// Exclusive bound (< or >)
    Exclusive(T),
}

/// Text type constraints
///
/// The `language` field determines what kind of text is expected:
/// - `None` - accepts any text (no language constraint)
/// - `Some("plaintext")` - expects plaintext (from `"..."` syntax or `Language::Plaintext`)
/// - `Some("rust")` - expects Rust code (from `` rust`...` `` syntax or `Language::Other("rust")`)
///
/// # Schema Syntax
///
/// - `.text` - any text (language=None)
/// - `.text.X` - text with language X (e.g., `.text.rust`, `.text.email`)
///
/// # Validation Rules
///
/// When validating a `Text` value against a `TextSchema`:
/// - `Language::Plaintext` matches schema with `language=None` or `language=Some("plaintext")`
/// - `Language::Implicit` matches any schema (the schema's language is applied)
/// - `Language::Other(lang)` matches schema with `language=None` or `language=Some(lang)`
///
/// ```eure
/// @variants.text
/// language = .text (optional)  # e.g., "rust", "email", "markdown"
/// min-length = .integer (optional)
/// max-length = .integer (optional)
/// pattern = .text (optional)
/// ```
#[derive(Debug, Clone, Default)]
pub struct TextSchema {
    /// Language identifier (e.g., "rust", "javascript", "email", "plaintext")
    ///
    /// - `None` - accepts any text regardless of language
    /// - `Some(lang)` - expects text with the specific language tag
    ///
    /// Note: When a value has `Language::Implicit` (from `` `...` `` syntax),
    /// it can be coerced to match the schema's expected language.
    pub language: Option<String>,
    /// Minimum length constraint (in UTF-8 code points)
    pub min_length: Option<u32>,
    /// Maximum length constraint (in UTF-8 code points)
    pub max_length: Option<u32>,
    /// Regex pattern constraint (applied to the text content).
    /// Pre-compiled at schema parse time for efficiency.
    pub pattern: Option<Regex>,
    /// Unknown fields (for future extensions like "flatten")
    pub unknown_fields: std::collections::HashMap<String, eure_document::document::NodeId>,
}

/// Integer type constraints
///
/// Spec: lines 360-364
/// ```eure
/// @variants.integer
/// range = .$types.range-string (optional)
/// multiple-of = .integer (optional)
/// ```
///
/// Note: Range string is parsed in the converter to Bound<BigInt>
#[derive(Debug, Clone, Default)]
pub struct IntegerSchema {
    /// Minimum value constraint (parsed from range string)
    pub min: Bound<BigInt>,
    /// Maximum value constraint (parsed from range string)
    pub max: Bound<BigInt>,
    /// Multiple-of constraint
    pub multiple_of: Option<BigInt>,
}

/// Float type constraints
///
/// Spec: lines 371-375
/// ```eure
/// @variants.float
/// range = .$types.range-string (optional)
/// multiple-of = .float (optional)
/// ```
///
/// Note: Range string is parsed in the converter to Bound<f64>
#[derive(Debug, Clone, Default)]
pub struct FloatSchema {
    /// Minimum value constraint (parsed from range string)
    pub min: Bound<f64>,
    /// Maximum value constraint (parsed from range string)
    pub max: Bound<f64>,
    /// Multiple-of constraint
    pub multiple_of: Option<f64>,
}

// ============================================================================
// Compound Type Schemas
// ============================================================================

/// Array type constraints
///
/// Spec: lines 426-439
/// ```eure
/// @variants.array
/// item = .$types.type
/// min-length = .integer (optional)
/// max-length = .integer (optional)
/// unique = .boolean (optional)
/// contains = .$types.type (optional)
/// $ext-type.binding-style = .$types.binding-style (optional)
/// ```
#[derive(Debug, Clone)]
pub struct ArraySchema {
    /// Schema for array elements (required)
    pub item: SchemaNodeId,
    /// Minimum number of elements
    pub min_length: Option<u32>,
    /// Maximum number of elements
    pub max_length: Option<u32>,
    /// All elements must be unique
    pub unique: bool,
    /// Array must contain at least one element matching this schema
    pub contains: Option<SchemaNodeId>,
    /// Binding style for formatting
    pub binding_style: Option<BindingStyle>,
}

/// Map type constraints
///
/// Spec: lines 453-459
/// ```eure
/// @variants.map
/// key = .$types.type
/// value = .$types.type
/// min-size = .integer (optional)
/// max-size = .integer (optional)
/// ```
#[derive(Debug, Clone)]
pub struct MapSchema {
    /// Schema for keys
    pub key: SchemaNodeId,
    /// Schema for values
    pub value: SchemaNodeId,
    /// Minimum number of key-value pairs
    pub min_size: Option<u32>,
    /// Maximum number of key-value pairs
    pub max_size: Option<u32>,
}

/// Record field with per-field metadata
///
/// Spec: lines 401-410 (value extensions)
/// ```eure
/// value.$ext-type.optional = .boolean (optional)
/// value.$ext-type.binding-style = .$types.binding-style (optional)
/// ```
#[derive(Debug, Clone)]
pub struct RecordFieldSchema {
    /// Schema for this field's value
    pub schema: SchemaNodeId,
    /// Field is optional (defaults to false = required)
    pub optional: bool,
    /// Binding style for this field
    pub binding_style: Option<BindingStyle>,
}

/// Record type with fixed named fields
///
/// Spec: lines 401-410
/// ```eure
/// @variants.record
/// $variant: map
/// key = .text
/// value = .$types.type
/// $ext-type.unknown-fields = .$types.unknown-fields-policy (optional)
/// ```
#[derive(Debug, Clone, Default)]
pub struct RecordSchema {
    /// Fixed field schemas (field name -> field schema with metadata)
    pub properties: HashMap<String, RecordFieldSchema>,
    /// Policy for unknown/additional fields (default: deny)
    pub unknown_fields: UnknownFieldsPolicy,
}

/// Policy for handling fields not defined in record properties
///
/// Spec: lines 240-251
/// ```eure
/// @ $types.unknown-fields-policy
/// @variants.deny = "deny"
/// @variants.allow = "allow"
/// @variants.schema = .$types.type
/// ```
#[derive(Debug, Clone, Default)]
pub enum UnknownFieldsPolicy {
    /// Deny unknown fields (default, strict)
    #[default]
    Deny,
    /// Allow any unknown fields without validation
    Allow,
    /// Unknown fields must match this schema
    Schema(SchemaNodeId),
}

/// Tuple type with fixed-length ordered elements
///
/// Spec: lines 465-468
/// ```eure
/// @variants.tuple
/// elements = [.$types.type]
/// $ext-type.binding-style = .$types.binding-style (optional)
/// ```
#[derive(Debug, Clone)]
pub struct TupleSchema {
    /// Schema for each element by position
    pub elements: Vec<SchemaNodeId>,
    /// Binding style for formatting
    pub binding_style: Option<BindingStyle>,
}

/// Union type with named variants
///
/// Spec: lines 415-423
/// ```eure
/// @variants.union
/// variants = { $variant: map, key => .text, value => .$types.type }
/// priority = [.text] (optional)
/// $ext-type.variant-repr = .$types.variant-repr (optional)
/// ```
#[derive(Debug, Clone)]
pub struct UnionSchema {
    /// Variant definitions (variant name -> schema)
    pub variants: HashMap<String, SchemaNodeId>,
    /// Priority order for variant matching in untagged unions
    /// First matching variant in priority order wins when multiple match
    pub priority: Option<Vec<String>>,
    /// Variant representation strategy (default: External)
    pub repr: VariantRepr,
}

// ============================================================================
// Binding Style
// ============================================================================

/// How to represent document paths in formatted output
///
/// Spec: lines 263-296
/// ```eure
/// @ $types.binding-style
/// $variant: union
/// variants { auto, passthrough, section, nested, binding, section-binding, section-root-binding }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BindingStyle {
    /// Automatically determine the best representation
    #[default]
    Auto,
    /// Pass through; defer to subsequent keys
    Passthrough,
    /// Create a new section (@ a.b.c)
    Section,
    /// Create a nested section (@ a.b.c { ... })
    Nested,
    /// Bind value (a.b.c = value)
    Binding,
    /// Section with block (a.b.c { ... })
    SectionBinding,
    /// Section with root binding (@ a.b.c = value)
    SectionRootBinding,
}

// ============================================================================
// Type Reference
// ============================================================================

/// Type reference (local or cross-schema)
///
/// Spec: lines 506-510
/// ```eure
/// @variants.ref
/// $variant: path
/// starts-with = .$types
/// length-min = 2
/// length-max = 3
/// ```
///
/// - Local reference (path length 2): `.$types.my-type`
/// - Cross-schema reference (path length 3): `.$types.namespace.type-name`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeReference {
    /// Namespace for cross-schema references (None for local refs)
    pub namespace: Option<String>,
    /// Type name
    pub name: Identifier,
}

// ============================================================================
// Metadata
// ============================================================================

/// Description can be plain string or markdown
///
/// Spec: lines 312-316
/// ```eure
/// description => { $variant: union, variants.string => .text, variants.markdown => .text.markdown }
/// ```
#[derive(Debug, Clone)]
pub enum Description {
    /// Plain text description
    String(String),
    /// Markdown formatted description
    Markdown(String),
}

/// Schema metadata (available at any nesting level via $ext-type on $types.type)
///
/// ```eure
/// description => union { string, .text.markdown } (optional)
/// deprecated => .boolean (optional)
/// default => .any (optional)
/// examples => [.text.eure] (optional)
/// ```
///
/// Note: `optional` and `binding_style` are per-field extensions stored in `RecordFieldSchema`
#[derive(Debug, Clone, Default)]
pub struct SchemaMetadata {
    /// Documentation/description
    pub description: Option<Description>,
    /// Marks as deprecated
    pub deprecated: bool,
    /// Default value for optional fields
    pub default: Option<EureDocument>,
    /// Example values in Eure code format
    pub examples: Option<Vec<String>>,
}

// ============================================================================
// Implementation
// ============================================================================

impl SchemaDocument {
    /// Create a new empty schema document
    pub fn new() -> Self {
        Self {
            nodes: vec![SchemaNode {
                content: SchemaNodeContent::Any,
                metadata: SchemaMetadata::default(),
                ext_types: HashMap::new(),
            }],
            root: SchemaNodeId(0),
            types: HashMap::new(),
        }
    }

    /// Get a reference to a node
    pub fn node(&self, id: SchemaNodeId) -> &SchemaNode {
        &self.nodes[id.0]
    }

    /// Get a mutable reference to a node
    pub fn node_mut(&mut self, id: SchemaNodeId) -> &mut SchemaNode {
        &mut self.nodes[id.0]
    }

    /// Create a new node and return its ID
    pub fn create_node(&mut self, content: SchemaNodeContent) -> SchemaNodeId {
        let id = SchemaNodeId(self.nodes.len());
        self.nodes.push(SchemaNode {
            content,
            metadata: SchemaMetadata::default(),
            ext_types: HashMap::new(),
        });
        id
    }

    /// Register a named type
    pub fn register_type(&mut self, name: Identifier, node_id: SchemaNodeId) {
        self.types.insert(name, node_id);
    }

    /// Look up a named type
    pub fn get_type(&self, name: &Identifier) -> Option<SchemaNodeId> {
        self.types.get(name).copied()
    }
}

impl Default for SchemaDocument {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Schema Reference
// ============================================================================

/// Reference to a schema file from `$schema` extension.
///
/// This type is used to extract the schema path from a document's root node.
/// The `$schema` extension specifies the path to the schema file that should
/// be used to validate the document.
///
/// # Example
///
/// ```eure
/// $schema = "./person.schema.eure"
/// name = "John"
/// age = 30
/// ```
#[derive(Debug, Clone)]
pub struct SchemaRef {
    /// Path to the schema file
    pub path: String,
    /// NodeId where the $schema was defined (for error reporting)
    pub node_id: eure_document::document::NodeId,
}
