#![no_std]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// Re-export commonly used types for eure! macro users
pub use text::Text;

/// A data structure for representing a Eure document without any span information.
pub mod tree;

/// Identifier type and parser.
pub mod identifier;

/// Unified text type for strings and code.
pub mod text;

/// A type-safe data-type of Eure data-model.
pub mod value;

/// A data structure for representing a Eure document including extensions.
pub mod document;

/// Data structure for representing a path in a Eure document.
pub mod path;

/// Data structure for representing a data-model of Eure.
pub mod data_model;

/// Trait for parsing Rust types from Eure documents.
pub mod parse;

#[cfg(feature = "std")]
pub use ahash::AHashMap as Map;
#[cfg(not(feature = "std"))]
pub type Map<K, V> = alloc::collections::BTreeMap<K, V>;

pub(crate) mod prelude_internal {
    #![allow(unused_imports)]
    #![allow(deprecated)]
    pub use crate::Map;
    pub use crate::data_model::*;
    pub use crate::document::constructor::DocumentConstructor;
    pub use crate::document::node::{Node, NodeMut, NodeValue};
    pub use crate::document::{EureDocument, InsertError, InsertErrorKind, NodeId};
    pub use crate::identifier::Identifier;
    pub use crate::path::{EurePath, PathSegment};
    pub use crate::text::{Language, SyntaxHint, Text, TextParseError};
    pub use crate::value::PrimitiveValue;
    pub use crate::value::{ObjectKey, Value};
    pub use alloc::boxed::Box;
    pub use alloc::{string::String, string::ToString, vec, vec::Vec};
    pub use thisisplural::Plural;
}

/// A declarative macro for building Eure documents, inspired by serde_json's `json!` macro.
///
/// # Syntax
///
/// The macro uses a TT muncher pattern to support arbitrary path combinations:
/// - Idents: `a.b.c`
/// - Extensions: `a.%ext` (use `%` instead of `$` since `$` is reserved in macros)
/// - Tuple index: `a.#0`, `a.#1`
/// - Array markers: `a[]` (push), `a[0]` (index)
/// - Tuple keys: `a.(1, "key")` (composite map keys)
/// - Mixed paths: `a.%ext[].b`, `a[].%ext.#0`, `a.(1, 2).name`
///
/// # Examples
///
/// ```
/// use eure_document::{eure, Text};
///
/// // Simple assignment
/// let doc = eure!({
///     name = "Alice",
///     age = 30,
/// });
///
/// // Nested paths
/// let doc = eure!({
///     user.name = "Bob",
///     user.active = true,
/// });
///
/// // Blocks (for grouping)
/// let doc = eure!({
///     user {
///         name = "Charlie",
///         role = "admin",
///     },
/// });
///
/// // Extensions
/// let doc = eure!({
///     field.%variant = Text::inline_implicit("text"),
/// });
///
/// // Tuple index
/// let doc = eure!({
///     point.#0 = 1.0f64,
///     point.#1 = 2.0f64,
/// });
///
/// // Array markers
/// let doc = eure!({
///     items[] = 1,
///     items[] = 2,
/// });
///
/// // Tuple keys (composite map keys)
/// let doc = eure!({
///     map.(1, "key") = "value",
///     map.(true, 2) = "another",
/// });
///
/// // Arrays (literal)
/// let doc = eure!({
///     tags = [Text::inline_implicit("a"), Text::inline_implicit("b")],
/// });
///
/// // Tuples (literal)
/// let doc = eure!({
///     point = (1.0f64, 2.0f64),
/// });
/// ```
#[macro_export]
macro_rules! eure {
    // ========================================================================
    // Entry points
    // ========================================================================

    // Empty document
    ({}) => {{
        $crate::document::EureDocument::new_empty()
    }};

    // Document with body
    ({ $($body:tt)* }) => {{
        let mut c = $crate::document::constructor::DocumentConstructor::new();
        $crate::eure!(@body c; $($body)*);
        c.finish()
    }};

    // ========================================================================
    // Body handlers
    // ========================================================================

    // Empty body
    (@body $c:ident;) => {};

    // Value binding at root: = value, ...
    (@body $c:ident; = $val:expr $(, $($tail:tt)*)?) => {{
        $c.bind_from($val).unwrap();
        $($crate::eure!(@body $c; $($tail)*);)?
    }};

    // Start parsing a statement - delegate to segment parser
    (@body $c:ident; $($tokens:tt)+) => {{
        let scope = $c.begin_scope();
        $crate::eure!(@parse_seg $c scope; $($tokens)+);
    }};

    // ========================================================================
    // Segment parsing - parse one path segment at a time
    // ========================================================================

    // Segment: ident
    (@parse_seg $c:ident $scope:ident; $seg:ident $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Ident(
            $crate::identifier::Identifier::new_unchecked(stringify!($seg))
        )).unwrap();
        $crate::eure!(@after_seg $c $scope; $($rest)*);
    }};

    // Segment: extension (%) with identifier
    (@parse_seg $c:ident $scope:ident; % $ext:ident $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Extension(
            $crate::identifier::Identifier::new_unchecked(stringify!($ext))
        )).unwrap();
        $crate::eure!(@after_seg $c $scope; $($rest)*);
    }};

    // Segment: extension (%) with string literal (for hyphenated names like "variant-repr")
    (@parse_seg $c:ident $scope:ident; % $ext:literal $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Extension(
            $ext.parse().unwrap()
        )).unwrap();
        $crate::eure!(@after_seg $c $scope; $($rest)*);
    }};

    // Segment: tuple index (#N)
    (@parse_seg $c:ident $scope:ident; # $idx:literal $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::TupleIndex($idx)).unwrap();
        $crate::eure!(@after_seg $c $scope; $($rest)*);
    }};

    // Segment: tuple key ((a, b, ...))
    (@parse_seg $c:ident $scope:ident; ($($tuple:tt)*) $($rest:tt)*) => {{
        let key = $crate::eure!(@build_tuple_key; $($tuple)*);
        $c.navigate($crate::path::PathSegment::Value(key)).unwrap();
        $crate::eure!(@after_seg $c $scope; $($rest)*);
    }};

    // Segment: string literal key ("key" for hyphenated identifiers)
    (@parse_seg $c:ident $scope:ident; $key:literal $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::Value($key.into())).unwrap();
        $crate::eure!(@after_seg $c $scope; $($rest)*);
    }};

    // ========================================================================
    // Build tuple key from contents
    // ========================================================================

    // Empty tuple
    (@build_tuple_key;) => {{
        $crate::value::ObjectKey::Tuple($crate::value::Tuple(Default::default()))
    }};

    // Non-empty tuple - items converted via Into<ObjectKey>
    (@build_tuple_key; $($item:expr),+ $(,)?) => {{
        $crate::value::ObjectKey::Tuple($crate::value::Tuple::from_iter(
            [$(<_ as Into<$crate::value::ObjectKey>>::into($item)),+]
        ))
    }};

    // ========================================================================
    // After segment - check for optional array marker
    // ========================================================================

    // Has array marker (captured as token tree)
    (@after_seg $c:ident $scope:ident; [$($arr:tt)*] $($rest:tt)*) => {{
        $crate::eure!(@handle_arr $c $scope [$($arr)*]; $($rest)*);
    }};

    // No array marker - go to after_arr
    (@after_seg $c:ident $scope:ident; $($rest:tt)*) => {{
        $crate::eure!(@after_arr $c $scope; $($rest)*);
    }};

    // ========================================================================
    // Handle array marker content
    // ========================================================================

    // Empty array marker (push)
    (@handle_arr $c:ident $scope:ident []; $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::ArrayIndex(None)).unwrap();
        $crate::eure!(@after_arr $c $scope; $($rest)*);
    }};

    // Array marker with index
    (@handle_arr $c:ident $scope:ident [$idx:literal]; $($rest:tt)*) => {{
        $c.navigate($crate::path::PathSegment::ArrayIndex(Some($idx))).unwrap();
        $crate::eure!(@after_arr $c $scope; $($rest)*);
    }};

    // ========================================================================
    // After array marker - check for continuation, assignment, or block
    // ========================================================================

    // Continuation: more path segments
    (@after_arr $c:ident $scope:ident; . $($rest:tt)+) => {{
        $crate::eure!(@parse_seg $c $scope; $($rest)+);
    }};

    // Terminal: array literal assignment
    (@after_arr $c:ident $scope:ident; = [$($items:expr),* $(,)?] $(, $($tail:tt)*)?) => {{
        $c.bind_empty_array().unwrap();
        $(
            let item_scope = $c.begin_scope();
            $c.navigate($crate::path::PathSegment::ArrayIndex(None)).unwrap();
            $c.bind_from($items).unwrap();
            $c.end_scope(item_scope).unwrap();
        )*
        $c.end_scope($scope).unwrap();
        $($crate::eure!(@body $c; $($tail)*);)?
    }};

    // Terminal: tuple literal assignment
    (@after_arr $c:ident $scope:ident; = ($($items:expr),* $(,)?) $(, $($tail:tt)*)?) => {{
        $c.bind_empty_tuple().unwrap();
        #[allow(unused_mut)]
        let mut _idx: u8 = 0;
        $(
            let item_scope = $c.begin_scope();
            $c.navigate($crate::path::PathSegment::TupleIndex(_idx)).unwrap();
            $c.bind_from($items).unwrap();
            $c.end_scope(item_scope).unwrap();
            _idx += 1;
        )*
        $c.end_scope($scope).unwrap();
        $($crate::eure!(@body $c; $($tail)*);)?
    }};

    // Terminal: object literal assignment (map with => syntax)
    (@after_arr $c:ident $scope:ident; = { $($key:expr => $val:expr),* $(,)? } $(, $($tail:tt)*)?) => {{
        $c.bind_empty_map().unwrap();
        $(
            let item_scope = $c.begin_scope();
            $c.navigate($crate::path::PathSegment::Value($key.into())).unwrap();
            $c.bind_from($val).unwrap();
            $c.end_scope(item_scope).unwrap();
        )*
        $c.end_scope($scope).unwrap();
        $($crate::eure!(@body $c; $($tail)*);)?
    }};

    // Terminal: simple assignment
    (@after_arr $c:ident $scope:ident; = $val:expr $(, $($tail:tt)*)?) => {{
        $c.bind_from($val).unwrap();
        $c.end_scope($scope).unwrap();
        $($crate::eure!(@body $c; $($tail)*);)?
    }};

    // Terminal: empty block -> empty map
    (@after_arr $c:ident $scope:ident; {} $(, $($tail:tt)*)?) => {{
        $c.bind_empty_map().unwrap();
        $c.end_scope($scope).unwrap();
        $($crate::eure!(@body $c; $($tail)*);)?
    }};

    // Terminal: non-empty block
    (@after_arr $c:ident $scope:ident; { $($inner:tt)+ } $(, $($tail:tt)*)?) => {{
        $crate::eure!(@body $c; $($inner)+);
        $c.end_scope($scope).unwrap();
        $($crate::eure!(@body $c; $($tail)*);)?
    }};
}

#[cfg(test)]
mod tests {
    use crate::document::EureDocument;
    use crate::text::Text;

    #[test]
    fn test_eure_empty() {
        let doc = eure!({});
        assert_eq!(doc, EureDocument::new_empty());
    }

    #[test]
    fn test_eure_simple_assignment() {
        let doc = eure!({
            name = "Alice",
        });

        // Verify the structure
        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let name_node_id = root.as_map().unwrap().get(&"name".into()).unwrap();
        let name_node = doc.node(name_node_id);
        let prim = name_node.as_primitive().unwrap();
        assert_eq!(prim.as_str(), Some("Alice"));
    }

    #[test]
    fn test_eure_nested_path() {
        let doc = eure!({
            user.name = "Bob",
            user.age = 30,
        });

        // Verify structure: root.user.name = "Bob", root.user.age = 30
        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let user_id = root.as_map().unwrap().get(&"user".into()).unwrap();
        let user = doc.node(user_id);
        let name_id = user.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("Bob"));

        let age_id = user.as_map().unwrap().get(&"age".into()).unwrap();
        let age = doc.node(age_id);
        assert!(matches!(
            age.as_primitive(),
            Some(crate::value::PrimitiveValue::Integer(_))
        ));
    }

    #[test]
    fn test_eure_block() {
        let doc = eure!({
            user {
                name = "Charlie",
                active = true,
            }
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let user_id = root.as_map().unwrap().get(&"user".into()).unwrap();
        let user = doc.node(user_id);
        let name_id = user.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("Charlie"));
    }

    #[test]
    fn test_eure_extension() {
        let doc = eure!({
            field.%variant = Text::inline_implicit("text"),
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let field_id = root.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);

        // Check extension
        let variant_id = field.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);
        let text = variant.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "text");
    }

    #[test]
    fn test_eure_extension_with_child() {
        // Test pattern: a.%ext.b = value
        let doc = eure!({
            field.%variant.name = Text::inline_implicit("text"),
            field.%variant.min_length = 3
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let field_id = root.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);

        // Check extension
        let variant_id = field.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);

        // Check child of extension
        let name_id = variant.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        let text = name.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "text");

        let min_length_id = variant.as_map().unwrap().get(&"min_length".into()).unwrap();
        let min_length = doc.node(min_length_id);
        assert!(matches!(
            min_length.as_primitive(),
            Some(crate::value::PrimitiveValue::Integer(_))
        ));
    }

    #[test]
    fn test_eure_array() {
        let doc = eure!({
            tags = [Text::inline_implicit("a"), Text::inline_implicit("b"), Text::inline_implicit("c")],
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let tags_id = root.as_map().unwrap().get(&"tags".into()).unwrap();
        let tags = doc.node(tags_id);
        let array = tags.as_array().unwrap();
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_eure_tuple() {
        let doc = eure!({
            point = (1.5f64, 2.5f64),
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let point_id = root.as_map().unwrap().get(&"point".into()).unwrap();
        let point = doc.node(point_id);
        let tuple = point.as_tuple().unwrap();
        assert_eq!(tuple.len(), 2);
    }

    #[test]
    fn test_eure_multiple_assignments() {
        let doc = eure!({
            a = 1,
            b = 2,
            c = 3,
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let map = root.as_map().unwrap();
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_eure_complex() {
        // A more complex example combining features
        let doc = eure!({
            schema {
                field.%variant = Text::inline_implicit("text"),
                field.min_length = 3,
                field.max_length = 20,
            },
            tags = [Text::inline_implicit("required")],
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);

        // Check schema exists
        let schema_id = root.as_map().unwrap().get(&"schema".into()).unwrap();
        let schema = doc.node(schema_id);

        // Check field exists with extension
        let field_id = schema.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);
        assert!(field.get_extension(&"variant".parse().unwrap()).is_some());

        // Check tags array
        let tags_id = root.as_map().unwrap().get(&"tags".into()).unwrap();
        let tags = doc.node(tags_id);
        assert_eq!(tags.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_eure_array_push() {
        // Test array push syntax: items[] = value
        let doc = eure!({
            items[] = 1,
            items[] = 2,
            items[] = 3,
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_eure_array_push_with_child() {
        // Test: items[].name = value (array push then navigate to child)
        let doc = eure!({
            items[].name = "first",
            items[].name = "second",
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 2);

        // Check first element has name = "first"
        let first_id = array.get(0).unwrap();
        let first = doc.node(first_id);
        let name_id = first.as_map().unwrap().get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("first"));
    }

    #[test]
    fn test_eure_tuple_index() {
        // Test tuple index syntax: point.#0, point.#1
        let doc = eure!({
            point.#0 = 1.5f64,
            point.#1 = 2.5f64,
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let point_id = root.as_map().unwrap().get(&"point".into()).unwrap();
        let point = doc.node(point_id);
        let tuple = point.as_tuple().unwrap();
        assert_eq!(tuple.len(), 2);
    }

    #[test]
    fn test_eure_mixed_path_extension_array() {
        // Test: a.%ext[].b = value
        let doc = eure!({
            field.%items[].name = "item1",
            field.%items[].name = "item2",
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let field_id = root.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);

        // Get extension
        let items_id = field.get_extension(&"items".parse().unwrap()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_eure_mixed_path_array_extension() {
        // Test: items[].%variant = value
        let doc = eure!({
            items[].%variant = Text::inline_implicit("text"),
            items[].%variant = Text::inline_implicit("number"),
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let items_id = root.as_map().unwrap().get(&"items".into()).unwrap();
        let items = doc.node(items_id);
        let array = items.as_array().unwrap();
        assert_eq!(array.len(), 2);

        // Check first element has extension
        let first_id = array.get(0).unwrap();
        let first = doc.node(first_id);
        let variant_id = first.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);
        assert_eq!(
            variant.as_primitive().unwrap().as_text().unwrap().as_str(),
            "text"
        );
    }

    #[test]
    fn test_eure_tuple_key() {
        use crate::value::{ObjectKey, Tuple};

        // Test tuple key: map.(1, "a") = value
        let doc = eure!({
            map.(1, "key") = "value1",
            map.(2, "key") = "value2",
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let map_id = root.as_map().unwrap().get(&"map".into()).unwrap();
        let map_node = doc.node(map_id);
        let map = map_node.as_map().unwrap();
        assert_eq!(map.len(), 2);

        // Check key (1, "key") exists
        let tuple_key = ObjectKey::Tuple(Tuple(alloc::vec![1.into(), "key".into()]));
        let value_id = map.get(&tuple_key).unwrap();
        let value = doc.node(value_id);
        assert_eq!(value.as_primitive().unwrap().as_str(), Some("value1"));
    }

    #[test]
    fn test_eure_tuple_key_with_bool() {
        use crate::value::{ObjectKey, Tuple};

        // Test tuple key with bool: map.(true, 1) = value
        let doc = eure!({
            map.(true, 1) = "yes",
            map.(false, 1) = "no",
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let map_id = root.as_map().unwrap().get(&"map".into()).unwrap();
        let map_node = doc.node(map_id);
        let map = map_node.as_map().unwrap();
        assert_eq!(map.len(), 2);

        // Check key (true, 1) exists
        let tuple_key = ObjectKey::Tuple(Tuple(alloc::vec![true.into(), 1.into()]));
        let value_id = map.get(&tuple_key).unwrap();
        let value = doc.node(value_id);
        assert_eq!(value.as_primitive().unwrap().as_str(), Some("yes"));
    }

    #[test]
    fn test_eure_tuple_key_with_child() {
        use crate::value::{ObjectKey, Tuple};

        // Test tuple key with child path: map.(1, 2).name = value
        let doc = eure!({
            map.(1, 2).name = "point_a",
            map.(1, 2).value = 42,
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let map_id = root.as_map().unwrap().get(&"map".into()).unwrap();
        let map_node = doc.node(map_id);
        let map = map_node.as_map().unwrap();

        // Check key (1, 2) has children
        let tuple_key = ObjectKey::Tuple(Tuple(alloc::vec![1.into(), 2.into()]));
        let entry_id = map.get(&tuple_key).unwrap();
        let entry = doc.node(entry_id);
        let entry_map = entry.as_map().unwrap();

        let name_id = entry_map.get(&"name".into()).unwrap();
        let name = doc.node(name_id);
        assert_eq!(name.as_primitive().unwrap().as_str(), Some("point_a"));
    }

    #[test]
    fn test_eure_string_key() {
        // Test string literal key for hyphenated identifiers: "min-length" = 3
        let doc = eure!({
            field."min-length" = 3,
            field."max-length" = 20,
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let field_id = root.as_map().unwrap().get(&"field".into()).unwrap();
        let field = doc.node(field_id);
        let field_map = field.as_map().unwrap();

        // Check "min-length" key exists
        let min_id = field_map.get(&"min-length".into()).unwrap();
        let min_node = doc.node(min_id);
        assert!(matches!(
            min_node.as_primitive(),
            Some(crate::value::PrimitiveValue::Integer(_))
        ));
    }

    #[test]
    fn test_eure_object_literal() {
        // Test object literal with => syntax
        let doc = eure!({
            variants.click = { "x" => 1.0f64, "y" => 2.0f64 },
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let variants_id = root.as_map().unwrap().get(&"variants".into()).unwrap();
        let variants = doc.node(variants_id);
        let click_id = variants.as_map().unwrap().get(&"click".into()).unwrap();
        let click = doc.node(click_id);
        let click_map = click.as_map().unwrap();

        assert_eq!(click_map.len(), 2);
        assert!(click_map.get(&"x".into()).is_some());
        assert!(click_map.get(&"y".into()).is_some());
    }

    #[test]
    fn test_eure_object_literal_with_text() {
        // Test object literal for schema-like patterns
        let doc = eure!({
            schema.variants.success = { "data" => Text::inline_implicit("any") },
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let schema_id = root.as_map().unwrap().get(&"schema".into()).unwrap();
        let schema = doc.node(schema_id);
        let variants_id = schema.as_map().unwrap().get(&"variants".into()).unwrap();
        let variants = doc.node(variants_id);
        let success_id = variants.as_map().unwrap().get(&"success".into()).unwrap();
        let success = doc.node(success_id);
        let success_map = success.as_map().unwrap();

        let data_id = success_map.get(&"data".into()).unwrap();
        let data = doc.node(data_id);
        let text = data.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "any");
    }

    #[test]
    fn test_eure_value_binding() {
        // Test value binding at root: = value
        let doc = eure!({
            = Text::inline_implicit("hello"),
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let text = root.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "hello");
    }

    #[test]
    fn test_eure_value_binding_with_extension() {
        // Test value binding with extension: = value, %ext = value
        let doc = eure!({
            = Text::inline_implicit("any"),
            %variant = "literal",
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);

        // Check value
        let text = root.as_primitive().unwrap().as_text().unwrap();
        assert_eq!(text.as_str(), "any");

        // Check extension
        let variant_id = root.get_extension(&"variant".parse().unwrap()).unwrap();
        let variant = doc.node(variant_id);
        assert_eq!(variant.as_primitive().unwrap().as_str(), Some("literal"));
    }

    #[test]
    fn test_eure_empty_block() {
        // Empty block should create an empty map, not a Hole
        let doc = eure!({
            config {},
        });

        let root_id = doc.get_root_id();
        let root = doc.node(root_id);
        let config_id = root.as_map().unwrap().get(&"config".into()).unwrap();
        let config = doc.node(config_id);

        // Should be an empty map, not Hole
        let map = config
            .as_map()
            .expect("Empty block should create an empty map");
        assert!(map.is_empty());
    }
}
