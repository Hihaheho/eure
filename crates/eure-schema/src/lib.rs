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

pub mod build;
pub mod convert;
pub mod identifiers;
pub mod parse;
pub mod synth;
pub mod to_source;
pub mod validate;

pub use build::{BuildSchema, SchemaBuilder};
pub use to_source::{ToSourceError, schema_to_source_document};

use eure_document::data_model::VariantRepr;
use eure_document::document::EureDocument;
use eure_document::identifier::Identifier;
use eure_macros::ParseDocument;
use indexmap::{IndexMap, IndexSet};
use num_bigint::BigInt;
use regex::Regex;

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
    pub types: IndexMap<Identifier, SchemaNodeId>,
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
    pub ext_types: IndexMap<Identifier, ExtTypeSchema>,
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

/// The kind of a schema node (discriminant without data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaKind {
    Any,
    Text,
    Integer,
    Float,
    Boolean,
    Null,
    Literal,
    Array,
    Map,
    Record,
    Tuple,
    Union,
    Reference,
}

impl std::fmt::Display for SchemaKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Any => "any",
            Self::Text => "text",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Null => "null",
            Self::Literal => "literal",
            Self::Array => "array",
            Self::Map => "map",
            Self::Record => "record",
            Self::Tuple => "tuple",
            Self::Union => "union",
            Self::Reference => "reference",
        };
        write!(f, "{}", name)
    }
}

impl SchemaNodeContent {
    /// Returns the kind of this schema node.
    pub fn kind(&self) -> SchemaKind {
        match self {
            Self::Any => SchemaKind::Any,
            Self::Text(_) => SchemaKind::Text,
            Self::Integer(_) => SchemaKind::Integer,
            Self::Float(_) => SchemaKind::Float,
            Self::Boolean => SchemaKind::Boolean,
            Self::Null => SchemaKind::Null,
            Self::Literal(_) => SchemaKind::Literal,
            Self::Array(_) => SchemaKind::Array,
            Self::Map(_) => SchemaKind::Map,
            Self::Record(_) => SchemaKind::Record,
            Self::Tuple(_) => SchemaKind::Tuple,
            Self::Union(_) => SchemaKind::Union,
            Self::Reference(_) => SchemaKind::Reference,
        }
    }
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

/// Range notation style for preserving original source format during roundtrip.
///
/// There are two ways to express ranges in Eure schema:
/// - Rust-style: `0..100`, `0..=100`, `0<..100`, etc.
/// - Interval notation: `[0, 100]`, `(0, 100)`, `[0, 100)`, etc.
///
/// This enum tracks which style was used in the source so it can be preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RangeStyle {
    /// Rust-style range: `0..100`, `0..=100`, etc. (default)
    #[default]
    Rust,
    /// Mathematical interval notation: `[0, 100]`, `(0, 100)`, etc.
    Interval,
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
#[derive(Debug, Clone, Default, ParseDocument)]
#[eure(crate = eure_document, rename_all = "kebab-case", allow_unknown_fields, allow_unknown_extensions)]
pub struct TextSchema {
    /// Language identifier (e.g., "rust", "javascript", "email", "plaintext")
    ///
    /// - `None` - accepts any text regardless of language
    /// - `Some(lang)` - expects text with the specific language tag
    ///
    /// Note: When a value has `Language::Implicit` (from `` `...` `` syntax),
    /// it can be coerced to match the schema's expected language.
    #[eure(default)]
    pub language: Option<String>,
    /// Minimum length constraint (in UTF-8 code points)
    #[eure(default)]
    pub min_length: Option<u32>,
    /// Maximum length constraint (in UTF-8 code points)
    #[eure(default)]
    pub max_length: Option<u32>,
    /// Regex pattern constraint (applied to the text content).
    /// Pre-compiled at schema parse time for efficiency.
    #[eure(default)]
    pub pattern: Option<Regex>,
}

impl PartialEq for TextSchema {
    fn eq(&self, other: &Self) -> bool {
        self.language == other.language
            && self.min_length == other.min_length
            && self.max_length == other.max_length
            && match (&self.pattern, &other.pattern) {
                (None, None) => true,
                (Some(a), Some(b)) => a.as_str() == b.as_str(),
                _ => false,
            }
    }
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
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IntegerSchema {
    /// Minimum value constraint (parsed from range string)
    pub min: Bound<BigInt>,
    /// Maximum value constraint (parsed from range string)
    pub max: Bound<BigInt>,
    /// Multiple-of constraint
    pub multiple_of: Option<BigInt>,
    /// Original range notation style for roundtrip preservation
    pub range_style: RangeStyle,
}

/// Float precision specifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FloatPrecision {
    /// 32-bit floating point (f32)
    F32,
    /// 64-bit floating point (f64) - default
    #[default]
    F64,
}

/// Float type constraints
///
/// Spec: lines 371-375
/// ```eure
/// @variants.float
/// range = .$types.range-string (optional)
/// multiple-of = .float (optional)
/// precision = "f32" | "f64" (optional, default: "f64")
/// ```
///
/// Note: Range string is parsed in the converter to Bound<f64>
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FloatSchema {
    /// Minimum value constraint (parsed from range string)
    pub min: Bound<f64>,
    /// Maximum value constraint (parsed from range string)
    pub max: Bound<f64>,
    /// Multiple-of constraint
    pub multiple_of: Option<f64>,
    /// Float precision (f32 or f64)
    pub precision: FloatPrecision,
    /// Original range notation style for roundtrip preservation
    pub range_style: RangeStyle,
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
    /// Whether to prefer inline/shorthand syntax (`[type]`) over explicit variant syntax
    /// (`{ $variant = "array", item = ... }`). Set based on original source format.
    /// - `None`: Auto-detect based on constraints (use inline if no constraints)
    /// - `Some(true)`: Prefer inline syntax
    /// - `Some(false)`: Prefer explicit variant syntax
    pub prefer_inline: Option<bool>,
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
    pub properties: IndexMap<String, RecordFieldSchema>,
    /// Schemas to flatten into this record.
    /// Each must point to a Record or Union schema.
    /// Fields from flattened schemas are merged into this record's field space.
    pub flatten: Vec<SchemaNodeId>,
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
/// $ext-type.variant-repr = .$types.variant-repr (optional)
/// ```
#[derive(Debug, Clone)]
pub struct UnionSchema {
    /// Variant definitions (variant name -> schema)
    pub variants: IndexMap<String, SchemaNodeId>,
    /// Variants that use unambiguous semantics (try all, detect conflicts).
    /// All other variants use short-circuit semantics (first match wins).
    pub unambiguous: IndexSet<String>,
    /// Variant representation strategy (default: Untagged)
    pub repr: VariantRepr,
    /// Whether repr was explicitly specified in the source (for roundtrip preservation)
    pub repr_explicit: bool,
    /// Variants that deny untagged matching (require explicit $variant)
    pub deny_untagged: IndexSet<String>,
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
#[derive(Debug, Clone, PartialEq, Eq, Default, ParseDocument)]
#[eure(crate = ::eure_document, rename_all = "kebab-case")]
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
/// - Local reference: `$types.my-type`
/// - Cross-schema reference: `$types.namespace.type-name`
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
#[derive(Debug, Clone, PartialEq, ParseDocument)]
#[eure(crate = eure_document, rename_all = "lowercase")]
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
/// examples => [`any`] (optional)
/// ```
///
/// Note: `optional` and `binding_style` are per-field extensions stored in `RecordFieldSchema`
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SchemaMetadata {
    /// Documentation/description
    pub description: Option<Description>,
    /// Marks as deprecated
    pub deprecated: bool,
    /// Default value for optional fields
    pub default: Option<EureDocument>,
    /// Example values as Eure documents
    pub examples: Option<Vec<EureDocument>>,
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
                ext_types: IndexMap::new(),
            }],
            root: SchemaNodeId(0),
            types: IndexMap::new(),
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
            ext_types: IndexMap::new(),
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

impl PartialEq for SchemaDocument {
    fn eq(&self, other: &Self) -> bool {
        // Compare types (must have same keys and structurally equal nodes)
        if self.types.len() != other.types.len() {
            return false;
        }
        for (name, &id1) in &self.types {
            match other.types.get(name) {
                Some(&id2) => {
                    if !self.nodes_equal(id1, other, id2) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        // Compare root nodes
        self.nodes_equal(self.root, other, other.root)
    }
}

impl SchemaDocument {
    /// Compare two nodes structurally, ignoring SchemaNodeId values
    pub fn nodes_equal(&self, id1: SchemaNodeId, other: &SchemaDocument, id2: SchemaNodeId) -> bool {
        let node1 = &self.nodes[id1.0];
        let node2 = &other.nodes[id2.0];

        // Compare metadata
        if node1.metadata != node2.metadata {
            return false;
        }

        // Compare ext_types
        if node1.ext_types.len() != node2.ext_types.len() {
            return false;
        }
        for (name, ext1) in &node1.ext_types {
            match node2.ext_types.get(name) {
                Some(ext2) => {
                    if ext1.optional != ext2.optional {
                        return false;
                    }
                    if !self.nodes_equal(ext1.schema, other, ext2.schema) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // Compare content
        self.contents_equal(&node1.content, other, &node2.content)
    }

    /// Compare two SchemaNodeContent values structurally
    fn contents_equal(
        &self,
        c1: &SchemaNodeContent,
        other: &SchemaDocument,
        c2: &SchemaNodeContent,
    ) -> bool {
        match (c1, c2) {
            (SchemaNodeContent::Any, SchemaNodeContent::Any) => true,
            (SchemaNodeContent::Boolean, SchemaNodeContent::Boolean) => true,
            (SchemaNodeContent::Null, SchemaNodeContent::Null) => true,
            (SchemaNodeContent::Text(t1), SchemaNodeContent::Text(t2)) => t1 == t2,
            (SchemaNodeContent::Integer(i1), SchemaNodeContent::Integer(i2)) => {
                i1.min == i2.min && i1.max == i2.max && i1.multiple_of == i2.multiple_of
                // Ignore range_style (formatting hint)
            }
            (SchemaNodeContent::Float(f1), SchemaNodeContent::Float(f2)) => {
                f1.min == f2.min
                    && f1.max == f2.max
                    && f1.multiple_of == f2.multiple_of
                    && f1.precision == f2.precision
                // Ignore range_style (formatting hint)
            }
            (SchemaNodeContent::Literal(l1), SchemaNodeContent::Literal(l2)) => l1 == l2,
            (SchemaNodeContent::Reference(r1), SchemaNodeContent::Reference(r2)) => r1 == r2,
            (SchemaNodeContent::Array(a1), SchemaNodeContent::Array(a2)) => {
                self.arrays_equal(a1, other, a2)
            }
            (SchemaNodeContent::Tuple(t1), SchemaNodeContent::Tuple(t2)) => {
                self.tuples_equal(t1, other, t2)
            }
            (SchemaNodeContent::Map(m1), SchemaNodeContent::Map(m2)) => {
                self.maps_equal(m1, other, m2)
            }
            (SchemaNodeContent::Record(r1), SchemaNodeContent::Record(r2)) => {
                self.records_equal(r1, other, r2)
            }
            (SchemaNodeContent::Union(u1), SchemaNodeContent::Union(u2)) => {
                self.unions_equal(u1, other, u2)
            }
            _ => false,
        }
    }

    fn arrays_equal(&self, a1: &ArraySchema, other: &SchemaDocument, a2: &ArraySchema) -> bool {
        // Ignore prefer_inline, binding_style (formatting hints)
        a1.min_length == a2.min_length
            && a1.max_length == a2.max_length
            && a1.unique == a2.unique
            && match (&a1.contains, &a2.contains) {
                (Some(c1), Some(c2)) => self.nodes_equal(*c1, other, *c2),
                (None, None) => true,
                _ => false,
            }
            && self.nodes_equal(a1.item, other, a2.item)
    }

    fn tuples_equal(&self, t1: &TupleSchema, other: &SchemaDocument, t2: &TupleSchema) -> bool {
        // Ignore binding_style (formatting hint)
        if t1.elements.len() != t2.elements.len() {
            return false;
        }
        t1.elements
            .iter()
            .zip(t2.elements.iter())
            .all(|(&e1, &e2)| self.nodes_equal(e1, other, e2))
    }

    fn maps_equal(&self, m1: &MapSchema, other: &SchemaDocument, m2: &MapSchema) -> bool {
        // Ignore min_size, max_size for semantic equality? No, those are constraints.
        m1.min_size == m2.min_size
            && m1.max_size == m2.max_size
            && self.nodes_equal(m1.key, other, m2.key)
            && self.nodes_equal(m1.value, other, m2.value)
    }

    fn records_equal(&self, r1: &RecordSchema, other: &SchemaDocument, r2: &RecordSchema) -> bool {
        if r1.properties.len() != r2.properties.len() {
            return false;
        }

        // Compare unknown_fields policy
        if !self.unknown_fields_equal(&r1.unknown_fields, other, &r2.unknown_fields) {
            return false;
        }

        // Compare flatten
        if r1.flatten.len() != r2.flatten.len() {
            return false;
        }
        for (&f1, &f2) in r1.flatten.iter().zip(r2.flatten.iter()) {
            if !self.nodes_equal(f1, other, f2) {
                return false;
            }
        }

        // Compare properties
        for (name, field1) in &r1.properties {
            match r2.properties.get(name) {
                Some(field2) => {
                    // Ignore binding_style (formatting hint)
                    if field1.optional != field2.optional {
                        return false;
                    }
                    if !self.nodes_equal(field1.schema, other, field2.schema) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }

    fn unknown_fields_equal(
        &self,
        p1: &UnknownFieldsPolicy,
        other: &SchemaDocument,
        p2: &UnknownFieldsPolicy,
    ) -> bool {
        match (p1, p2) {
            (UnknownFieldsPolicy::Deny, UnknownFieldsPolicy::Deny) => true,
            (UnknownFieldsPolicy::Allow, UnknownFieldsPolicy::Allow) => true,
            (UnknownFieldsPolicy::Schema(s1), UnknownFieldsPolicy::Schema(s2)) => {
                self.nodes_equal(*s1, other, *s2)
            }
            _ => false,
        }
    }

    fn unions_equal(&self, u1: &UnionSchema, other: &SchemaDocument, u2: &UnionSchema) -> bool {
        // Ignore repr_explicit (formatting hint)
        if u1.repr != u2.repr {
            return false;
        }
        if u1.variants.len() != u2.variants.len() {
            return false;
        }
        if u1.unambiguous != u2.unambiguous {
            return false;
        }
        if u1.deny_untagged != u2.deny_untagged {
            return false;
        }
        for (name, &v1) in &u1.variants {
            match u2.variants.get(name) {
                Some(&v2) => {
                    if !self.nodes_equal(v1, other, v2) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
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
