//! EURE Schema validation library
//!
//! This library provides schema extraction and validation for EURE documents.
//! It supports both standalone schema files and inline schemas within documents.

pub mod convert;
pub mod identifiers;

use eure_value::data_model::VariantRepr;
use eure_value::identifier::Identifier;
use eure_value::path::EurePath;
use eure_value::value::PrimitiveValue;
use num_bigint::BigInt;
use std::collections::HashMap;

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

/// Reference to a schema node by index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchemaNodeId(pub usize);

/// A single schema node
#[derive(Debug, Clone)]
pub struct SchemaNode {
    /// The type definition, structure, and constraints
    pub content: SchemaNodeContent,
    /// Metadata that doesn't directly affect validation
    pub metadata: SchemaMetadata,
}

/// Type definitions with their specific constraints
#[derive(Debug, Clone)]
pub enum SchemaNodeContent {
    /// No constraints (.any)
    Any,

    // --- Primitives ---
    /// String type with constraints
    String(StringSchema),
    /// Code type (separate from String)
    Code(CodeSchema),
    /// Integer type with BigInt support
    Integer(IntegerSchema),
    /// Float type
    Float(FloatSchema),
    /// Boolean type
    Boolean(BooleanSchema),
    /// Null type
    Null,
    /// Path type (.string, .user.name, etc.)
    Path(PathSchema),

    // --- Compounds ---
    /// Array type ($array)
    Array(ArraySchema),
    /// Map type (dynamic K -> V)
    Map(MapSchema),
    /// Record type (fixed fields, like struct)
    Record(RecordSchema),
    /// Tuple type
    Tuple(TupleSchema),

    // --- Logic ---
    /// Union type ($union)
    Union(Vec<SchemaNodeId>),
    /// Variant type (tagged union, $variants)
    Variant(VariantSchema),

    // --- Reference ---
    /// Type reference (.$types.Person)
    Reference(Identifier),
}

// --- Boundary Condition (ADT for proper modeling) ---

/// Boundary condition for numeric constraints
/// Uses ADT to prevent invalid states (e.g., both inclusive and exclusive)
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Bound<T> {
    /// No constraint (-∞ or +∞)
    #[default]
    Unbounded,
    /// Greater than or equal to / Less than or equal to
    Inclusive(T),
    /// Greater than / Less than
    Exclusive(T),
}

// --- Primitive Type Schemas ---

#[derive(Debug, Clone, Default)]
pub struct StringSchema {
    /// Length constraint (min, max)
    pub length: Option<(u32, u32)>,
    /// Regex pattern constraint
    pub pattern: Option<String>,
    /// Format constraint (e.g., "email", "url")
    pub format: Option<String>,
    /// Constant value (literal)
    pub r#const: Option<String>,
    /// Enumeration of allowed values
    pub r#enum: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct CodeSchema {
    /// Language identifier (.code.rust, .code.javascript, etc.)
    pub language: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct IntegerSchema {
    /// Minimum value constraint
    pub min: Bound<BigInt>,
    /// Maximum value constraint
    pub max: Bound<BigInt>,
    /// Multiple of constraint
    pub multiple_of: Option<BigInt>,
    /// Constant value (literal)
    pub r#const: Option<BigInt>,
    /// Enumeration of allowed values
    pub r#enum: Option<Vec<BigInt>>,
}

#[derive(Debug, Clone, Default)]
pub struct FloatSchema {
    /// Minimum value constraint
    pub min: Bound<f64>,
    /// Maximum value constraint
    pub max: Bound<f64>,
    /// Constant value (literal)
    pub r#const: Option<f64>,
    /// Enumeration of allowed values
    pub r#enum: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Default)]
pub struct BooleanSchema {
    /// Constant value (literal true or false)
    pub r#const: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct PathSchema {
    /// Path-specific constraints (for future extension)
    /// e.g., must be absolute path, must start with specific root, etc.
    pub r#const: Option<EurePath>,
}

// --- Compound Type Schemas ---

#[derive(Debug, Clone)]
pub struct ArraySchema {
    /// Schema for array elements (required)
    pub item: SchemaNodeId,
    /// Minimum number of items
    pub min_items: Option<u32>,
    /// Maximum number of items
    pub max_items: Option<u32>,
    /// All items must be unique
    pub unique: bool,
    /// Array must contain this value
    pub contains: Option<PrimitiveValue>,
}

#[derive(Debug, Clone)]
pub struct MapSchema {
    /// Schema for keys
    pub key: SchemaNodeId,
    /// Schema for values
    pub value: SchemaNodeId,
    /// Minimum number of key-value pairs
    pub min_pairs: Option<u32>,
    /// Maximum number of key-value pairs
    pub max_pairs: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct RecordSchema {
    /// Fixed field schemas
    pub properties: HashMap<String, SchemaNodeId>,
    /// Policy for unknown/additional fields
    pub unknown_fields: UnknownFieldsPolicy,
}

/// Policy for handling fields not defined in properties
#[derive(Debug, Clone, Default)]
pub enum UnknownFieldsPolicy {
    /// Deny unknown fields (default, strict)
    #[default]
    Deny,
    /// Allow any unknown fields
    Allow,
    /// Unknown fields must match this schema
    Schema(SchemaNodeId),
}

#[derive(Debug, Clone)]
pub struct TupleSchema {
    /// Schema for each tuple element by position
    pub items: Vec<SchemaNodeId>,
}

// --- Logic Type Schemas ---

#[derive(Debug, Clone)]
pub struct VariantSchema {
    /// Variant definitions (variant name -> schema)
    pub variants: HashMap<String, SchemaNodeId>,
    /// Variant representation strategy ($variant-repr)
    pub repr: VariantRepr,
}

// --- Metadata ---

/// Metadata that doesn't directly affect validation
#[derive(Debug, Clone, Default)]
pub struct SchemaMetadata {
    /// Field is optional ($optional)
    pub optional: bool,
    /// Documentation/description
    pub description: Option<String>,
    /// Deprecated flag
    pub deprecated: bool,
    /// Default value
    pub default: Option<PrimitiveValue>,

    // Editor/Formatter hints
    /// Prefer section syntax ($prefer.section)
    pub prefer_section: bool,
    /// Rename for serialization ($rename)
    pub rename: Option<String>,
    /// Naming convention for all fields ($rename-all)
    pub rename_all: Option<String>,
}

// --- Implementation ---

impl SchemaDocument {
    /// Create a new empty schema document
    pub fn new() -> Self {
        Self {
            nodes: vec![SchemaNode {
                content: SchemaNodeContent::Any,
                metadata: SchemaMetadata::default(),
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
