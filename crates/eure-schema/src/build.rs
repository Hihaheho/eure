//! Schema building from Rust types
//!
//! This module provides the [`BuildSchema`] trait and [`SchemaBuilder`] for
//! generating schema definitions from Rust types, either manually or via derive.
//!
//! # Example
//!
//! ```ignore
//! use eure_schema::{BuildSchema, SchemaDocument};
//!
//! #[derive(BuildSchema)]
//! #[eure(type_name = "user")]
//! struct User {
//!     name: String,
//!     age: Option<u32>,
//! }
//!
//! let schema = SchemaDocument::of::<User>();
//! ```

use std::any::TypeId;
use std::collections::HashMap;

use eure_document::Text;
use indexmap::IndexMap;

use crate::{
    SchemaDocument, SchemaMetadata, SchemaNode, SchemaNodeContent, SchemaNodeId, TextSchema,
};

/// Trait for types that can build their schema representation.
///
/// This trait is typically derived using `#[derive(BuildSchema)]`, but can also
/// be implemented manually for custom schema generation.
///
/// # Type Registration
///
/// Types can optionally provide a `type_name()` to register themselves in the
/// schema's `$types` namespace. This is useful for:
/// - Creating reusable type definitions
/// - Enabling type references across the schema
/// - Providing meaningful names in generated schemas
///
/// Primitive types typically return `None` for `type_name()`.
pub trait BuildSchema {
    /// The type name for registration in `$types` namespace.
    ///
    /// Return `Some("my-type")` to register this type as `$types.my-type`.
    /// Return `None` (default) for inline/anonymous types.
    fn type_name() -> Option<&'static str> {
        None
    }

    /// Build the schema content for this type.
    ///
    /// Use `ctx.build::<T>()` for nested types - this handles caching
    /// and recursion automatically.
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent;

    /// Optional metadata for this type's schema node.
    ///
    /// Override to provide description, deprecation status, defaults, or examples.
    fn schema_metadata() -> SchemaMetadata {
        SchemaMetadata::default()
    }
}

/// Builder for constructing schema documents from Rust types.
///
/// The builder maintains:
/// - An arena of schema nodes
/// - A cache by `TypeId` to prevent duplicate definitions and handle recursion
/// - Type registrations for the `$types` namespace
pub struct SchemaBuilder {
    /// The schema document being built
    doc: SchemaDocument,
    /// Cache of built types by TypeId (prevents duplicates, handles recursion)
    cache: HashMap<TypeId, SchemaNodeId>,
}

impl SchemaBuilder {
    /// Create a new schema builder.
    pub fn new() -> Self {
        Self {
            doc: SchemaDocument {
                nodes: Vec::new(),
                root: SchemaNodeId(0), // Will be set in finish()
                types: Default::default(),
            },
            cache: HashMap::new(),
        }
    }

    /// Build the schema for type `T`, with caching and recursion handling.
    ///
    /// This is the primary method for building nested types. It:
    /// 1. Returns cached ID if already built (idempotent)
    /// 2. Reserves a node slot before building (handles recursion)
    /// 3. Calls `T::build_schema()` to get the content
    /// 4. For named types: registers in $types and returns a Reference node
    pub fn build<T: BuildSchema + 'static>(&mut self) -> SchemaNodeId {
        let type_id = TypeId::of::<T>();

        // Return cached if already built
        if let Some(&id) = self.cache.get(&type_id) {
            return id;
        }

        // Check if this type has a name (for registration)
        let type_name = T::type_name();

        // For named types, we need two nodes: content + reference
        // For unnamed types, just the content node
        if let Some(name) = type_name {
            // Reserve a slot for the content node
            let content_id = self.reserve_node();

            // Build the schema content
            let content = T::build_schema(self);
            let metadata = T::schema_metadata();
            self.set_node(content_id, content, metadata);

            // Register the type
            if let Ok(ident) = name.parse::<eure_document::identifier::Identifier>() {
                self.doc.types.insert(ident, content_id);
            }

            // Create a Reference node that points to this type
            let ref_id = self.create_node(SchemaNodeContent::Reference(crate::TypeReference {
                namespace: None,
                name: name.parse().expect("valid type name"),
            }));

            // Cache the reference ID so subsequent calls return the reference
            self.cache.insert(type_id, ref_id);
            ref_id
        } else {
            // Unnamed type: just build and cache the content node
            let id = self.reserve_node();
            self.cache.insert(type_id, id);

            let content = T::build_schema(self);
            let metadata = T::schema_metadata();
            self.set_node(id, content, metadata);

            id
        }
    }

    /// Create a schema node with the given content.
    ///
    /// Use this for creating anonymous/inline nodes that don't need caching.
    /// For types that implement `BuildSchema`, prefer `build::<T>()`.
    pub fn create_node(&mut self, content: SchemaNodeContent) -> SchemaNodeId {
        let id = SchemaNodeId(self.doc.nodes.len());
        self.doc.nodes.push(SchemaNode {
            content,
            metadata: SchemaMetadata::default(),
            ext_types: Default::default(),
        });
        id
    }

    /// Create a schema node with content and metadata.
    pub fn create_node_with_metadata(
        &mut self,
        content: SchemaNodeContent,
        metadata: SchemaMetadata,
    ) -> SchemaNodeId {
        let id = SchemaNodeId(self.doc.nodes.len());
        self.doc.nodes.push(SchemaNode {
            content,
            metadata,
            ext_types: Default::default(),
        });
        id
    }

    /// Reserve a node slot, returning its ID.
    ///
    /// The node is initialized with `Any` content and must be finalized
    /// with `set_node()` before the schema is complete.
    fn reserve_node(&mut self) -> SchemaNodeId {
        let id = SchemaNodeId(self.doc.nodes.len());
        self.doc.nodes.push(SchemaNode {
            content: SchemaNodeContent::Any, // Placeholder
            metadata: SchemaMetadata::default(),
            ext_types: Default::default(),
        });
        id
    }

    /// Set the content and metadata of a reserved node.
    fn set_node(&mut self, id: SchemaNodeId, content: SchemaNodeContent, metadata: SchemaMetadata) {
        let node = &mut self.doc.nodes[id.0];
        node.content = content;
        node.metadata = metadata;
    }

    /// Get mutable access to a node for adding ext_types or modifying metadata.
    pub fn node_mut(&mut self, id: SchemaNodeId) -> &mut SchemaNode {
        &mut self.doc.nodes[id.0]
    }

    /// Register a named type in the `$types` namespace.
    pub fn register_type(&mut self, name: &str, id: SchemaNodeId) {
        if let Ok(ident) = name.parse() {
            self.doc.types.insert(ident, id);
        }
    }

    /// Consume the builder and produce the final schema document.
    pub fn finish(mut self, root: SchemaNodeId) -> SchemaDocument {
        self.doc.root = root;
        self.doc
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaDocument {
    /// Generate a schema document for type `T`.
    ///
    /// This is the main entry point for schema generation from Rust types.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use eure_schema::SchemaDocument;
    ///
    /// let schema = SchemaDocument::of::<MyType>();
    /// ```
    pub fn of<T: BuildSchema + 'static>() -> SchemaDocument {
        let mut builder = SchemaBuilder::new();
        let root = builder.build::<T>();
        builder.finish(root)
    }
}

// ============================================================================
// Primitive Type Implementations
// ============================================================================

impl BuildSchema for String {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Text(crate::TextSchema::default())
    }
}

impl BuildSchema for &str {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Text(crate::TextSchema::default())
    }
}

impl BuildSchema for bool {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Boolean
    }
}

// Signed integers
impl BuildSchema for i8 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for i16 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for i32 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for i64 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for i128 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for isize {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

// Unsigned integers
impl BuildSchema for u8 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for u16 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for u32 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for u64 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for u128 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

impl BuildSchema for usize {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Integer(crate::IntegerSchema::default())
    }
}

// Floats
impl BuildSchema for f32 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Float(crate::FloatSchema {
            precision: crate::FloatPrecision::F32,
            ..Default::default()
        })
    }
}

impl BuildSchema for f64 {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Float(crate::FloatSchema {
            precision: crate::FloatPrecision::F64,
            ..Default::default()
        })
    }
}

// Unit type
impl BuildSchema for () {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Null
    }
}

impl BuildSchema for Text {
    fn build_schema(_ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        SchemaNodeContent::Text(TextSchema {
            language: None,
            min_length: None,
            max_length: None,
            pattern: None,
            unknown_fields: IndexMap::new(),
        })
    }
}

// ============================================================================
// Compound Type Implementations
// ============================================================================

/// Option<T> is represented as a union: some(T) | none(null)
impl<T: BuildSchema + 'static> BuildSchema for Option<T> {
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let some_schema = ctx.build::<T>();
        let none_schema = ctx.create_node(SchemaNodeContent::Null);

        SchemaNodeContent::Union(crate::UnionSchema {
            variants: IndexMap::from([
                ("some".to_string(), some_schema),
                ("none".to_string(), none_schema),
            ]),
            unambiguous: Default::default(),
            repr: eure_document::data_model::VariantRepr::default(),
            deny_untagged: Default::default(),
        })
    }
}

/// Result<T, E> is represented as a union: ok(T) | err(E)
impl<T: BuildSchema + 'static, E: BuildSchema + 'static> BuildSchema for Result<T, E> {
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let ok_schema = ctx.build::<T>();
        let err_schema = ctx.build::<E>();

        SchemaNodeContent::Union(crate::UnionSchema {
            variants: IndexMap::from([
                ("ok".to_string(), ok_schema),
                ("err".to_string(), err_schema),
            ]),
            unambiguous: Default::default(),
            repr: eure_document::data_model::VariantRepr::default(),
            deny_untagged: Default::default(),
        })
    }
}

/// Vec<T> is represented as an array with item type T
impl<T: BuildSchema + 'static> BuildSchema for Vec<T> {
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let item = ctx.build::<T>();
        SchemaNodeContent::Array(crate::ArraySchema {
            item,
            min_length: None,
            max_length: None,
            unique: false,
            contains: None,
            binding_style: None,
        })
    }
}

/// HashMap<K, V> is represented as a map
impl<K: BuildSchema + 'static, V: BuildSchema + 'static> BuildSchema
    for std::collections::HashMap<K, V>
{
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let key = ctx.build::<K>();
        let value = ctx.build::<V>();
        SchemaNodeContent::Map(crate::MapSchema {
            key,
            value,
            min_size: None,
            max_size: None,
        })
    }
}

/// BTreeMap<K, V> is represented as a map
impl<K: BuildSchema + 'static, V: BuildSchema + 'static> BuildSchema
    for std::collections::BTreeMap<K, V>
{
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let key = ctx.build::<K>();
        let value = ctx.build::<V>();
        SchemaNodeContent::Map(crate::MapSchema {
            key,
            value,
            min_size: None,
            max_size: None,
        })
    }
}

/// Box<T> delegates to T
impl<T: BuildSchema + 'static> BuildSchema for Box<T> {
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        T::build_schema(ctx)
    }
}

/// Rc<T> delegates to T
impl<T: BuildSchema + 'static> BuildSchema for std::rc::Rc<T> {
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        T::build_schema(ctx)
    }
}

/// Arc<T> delegates to T
impl<T: BuildSchema + 'static> BuildSchema for std::sync::Arc<T> {
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        T::build_schema(ctx)
    }
}

// Tuples
impl<A: BuildSchema + 'static> BuildSchema for (A,) {
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let elements = vec![ctx.build::<A>()];
        SchemaNodeContent::Tuple(crate::TupleSchema {
            elements,
            binding_style: None,
        })
    }
}

impl<A: BuildSchema + 'static, B: BuildSchema + 'static> BuildSchema for (A, B) {
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let elements = vec![ctx.build::<A>(), ctx.build::<B>()];
        SchemaNodeContent::Tuple(crate::TupleSchema {
            elements,
            binding_style: None,
        })
    }
}

impl<A: BuildSchema + 'static, B: BuildSchema + 'static, C: BuildSchema + 'static> BuildSchema
    for (A, B, C)
{
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let elements = vec![ctx.build::<A>(), ctx.build::<B>(), ctx.build::<C>()];
        SchemaNodeContent::Tuple(crate::TupleSchema {
            elements,
            binding_style: None,
        })
    }
}

impl<
    A: BuildSchema + 'static,
    B: BuildSchema + 'static,
    C: BuildSchema + 'static,
    D: BuildSchema + 'static,
> BuildSchema for (A, B, C, D)
{
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let elements = vec![
            ctx.build::<A>(),
            ctx.build::<B>(),
            ctx.build::<C>(),
            ctx.build::<D>(),
        ];
        SchemaNodeContent::Tuple(crate::TupleSchema {
            elements,
            binding_style: None,
        })
    }
}

impl<
    A: BuildSchema + 'static,
    B: BuildSchema + 'static,
    C: BuildSchema + 'static,
    D: BuildSchema + 'static,
    E: BuildSchema + 'static,
> BuildSchema for (A, B, C, D, E)
{
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let elements = vec![
            ctx.build::<A>(),
            ctx.build::<B>(),
            ctx.build::<C>(),
            ctx.build::<D>(),
            ctx.build::<E>(),
        ];
        SchemaNodeContent::Tuple(crate::TupleSchema {
            elements,
            binding_style: None,
        })
    }
}

impl<
    A: BuildSchema + 'static,
    B: BuildSchema + 'static,
    C: BuildSchema + 'static,
    D: BuildSchema + 'static,
    E: BuildSchema + 'static,
    F: BuildSchema + 'static,
> BuildSchema for (A, B, C, D, E, F)
{
    fn build_schema(ctx: &mut SchemaBuilder) -> SchemaNodeContent {
        let elements = vec![
            ctx.build::<A>(),
            ctx.build::<B>(),
            ctx.build::<C>(),
            ctx.build::<D>(),
            ctx.build::<E>(),
            ctx.build::<F>(),
        ];
        SchemaNodeContent::Tuple(crate::TupleSchema {
            elements,
            binding_style: None,
        })
    }
}
