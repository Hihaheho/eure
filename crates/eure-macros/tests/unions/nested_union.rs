//! Tests for nested generic unions.
//!
//! These tests cover patterns like `Item<TextOrNested<Level2>>` where
//! unions contain other unions as type parameters. Inspired by blog.eure.dev's
//! article structure which uses nested union patterns.

use eure::ParseDocument;

/// An item can be either a single value or a list of values.
/// Uses lowercase variant names like JSON/YAML conventions.
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all = "lowercase")]
enum Item<T> {
    Normal(T),
    List(Vec<T>),
}

/// A value can be either nested (containing a struct) or plain text.
/// Uses lowercase variant names.
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all = "lowercase")]
enum TextOrNested<T> {
    Nested(T),
    Text(String),
}

/// A simple leaf struct for nesting.
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct Level2 {
    title: String,
    value: i32,
}

// =============================================================================
// Item<TextOrNested<Level2>> tests
// =============================================================================

#[test]
fn test_item_normal_text_or_nested_text() {
    use eure::eure;

    // A simple string should be parsed as Item::Normal(TextOrNested::Text("hello"))
    let doc = eure!({ = "hello" });
    let result: Item<TextOrNested<Level2>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::Normal(TextOrNested::Text("hello".to_string()))
    );
}

#[test]
fn test_item_normal_text_or_nested_nested() {
    use eure::eure;

    // A struct should be parsed as Item::Normal(TextOrNested::Nested(Level2 {...}))
    let doc = eure!({
        title = "Test"
        value = 42
    });
    let result: Item<TextOrNested<Level2>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::Normal(TextOrNested::Nested(Level2 {
            title: "Test".to_string(),
            value: 42,
        }))
    );
}

#[test]
fn test_item_list_text_or_nested_all_text() {
    use eure::eure;

    // A list of strings should be parsed as Item::List([TextOrNested::Text(...), ...])
    let doc = eure!({ = ["a", "b", "c"] });
    let result: Item<TextOrNested<Level2>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::List(vec![
            TextOrNested::Text("a".to_string()),
            TextOrNested::Text("b".to_string()),
            TextOrNested::Text("c".to_string()),
        ])
    );
}

// =============================================================================
// Explicit $variant path tests for nested unions
// =============================================================================

#[test]
fn test_nested_union_variant_path_normal_text() {
    use eure::eure;

    // Explicit $variant = "normal.text" should work
    let doc = eure!({
        = "hello"
        %variant = "normal.text"
    });
    let result: Item<TextOrNested<Level2>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::Normal(TextOrNested::Text("hello".to_string()))
    );
}

#[test]
fn test_nested_union_variant_path_normal_nested() {
    use eure::eure;

    let doc = eure!({
        %variant = "normal.nested"
        title = "Explicit"
        value = 99
    });
    let result: Item<TextOrNested<Level2>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::Normal(TextOrNested::Nested(Level2 {
            title: "Explicit".to_string(),
            value: 99,
        }))
    );
}

#[test]
fn test_nested_union_variant_path_list() {
    use eure::eure;

    // Explicit $variant = "list" should work (inner elements use untagged)
    let doc = eure!({
        = ["a", "b"]
        %variant = "list"
    });
    let result: Item<TextOrNested<Level2>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::List(vec![
            TextOrNested::Text("a".to_string()),
            TextOrNested::Text("b".to_string()),
        ])
    );
}

// =============================================================================
// Triple nested union tests
// =============================================================================

/// Third level of nesting for testing deep union hierarchies.
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, rename_all = "lowercase")]
enum DeepChoice {
    Leaf(String),
    Number(i32),
}

#[test]
fn test_triple_nested_union_untagged() {
    use eure::eure;

    // Item<TextOrNested<DeepChoice>> - three levels of nesting
    // String input matches the deepest variant first in untagged mode:
    // Item::Normal -> TextOrNested::Nested -> DeepChoice::Leaf
    // (because "Nested" is tried before "Text", and "Leaf" accepts String)
    let doc = eure!({ = "leaf value" });
    let result: Item<TextOrNested<DeepChoice>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::Normal(TextOrNested::Nested(DeepChoice::Leaf(
            "leaf value".to_string()
        )))
    );
}

#[test]
fn test_triple_nested_union_variant_path_three_levels() {
    use eure::eure;

    // Explicit $variant = "normal.nested.leaf" for three-level path
    let doc = eure!({
        = "deep leaf"
        %variant = "normal.nested.leaf"
    });
    let result: Item<TextOrNested<DeepChoice>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::Normal(TextOrNested::Nested(DeepChoice::Leaf(
            "deep leaf".to_string()
        )))
    );
}

#[test]
fn test_triple_nested_union_variant_path_number() {
    use eure::eure;

    // Explicit $variant = "normal.nested.number" for three-level path
    let doc = eure!({
        = 42
        %variant = "normal.nested.number"
    });
    let result: Item<TextOrNested<DeepChoice>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::Normal(TextOrNested::Nested(DeepChoice::Number(42)))
    );
}

// =============================================================================
// Simple nested union (two levels, same as generics.rs but different patterns)
// =============================================================================

/// A simpler nested union test using the existing Item enum
#[test]
fn test_item_nested_in_list_with_numbers() {
    use eure::eure;

    // List of numbers
    let doc = eure!({ = [1, 2, 3] });
    let result: Item<i32> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Item::List(vec![1, 2, 3]));
}

/// Either<L, R> nested inside Item<T>
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
enum Either<L, R> {
    Left(L),
    Right(R),
}

#[test]
fn test_item_of_either_normal_left() {
    use eure::eure;

    // String input -> Item::Normal(Either::Left("hello"))
    let doc = eure!({ = "hello" });
    let result: Item<Either<String, i32>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Item::Normal(Either::Left("hello".to_string())));
}

#[test]
fn test_item_of_either_normal_right() {
    use eure::eure;

    // Integer input -> Item::Normal(Either::Right(42))
    let doc = eure!({ = 42 });
    let result: Item<Either<String, i32>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result, Item::Normal(Either::Right(42)));
}

#[test]
fn test_item_of_either_list() {
    use eure::eure;

    // List of strings -> Item::List([Either::Left("a"), Either::Left("b")])
    let doc = eure!({ = ["a", "b"] });
    let result: Item<Either<String, i32>> = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Item::List(vec![
            Either::Left("a".to_string()),
            Either::Left("b".to_string()),
        ])
    );
}
