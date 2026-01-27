//! Tests for flatten with nested unions.
//!
//! BUG: parse_map! doesn't mark fields as accessed, causing deny_unknown_fields to fail.

use eure::ParseDocument;
use indexmap::IndexMap;

// =============================================================================
// Minimal reproduction: flatten with IndexMap
// =============================================================================

#[derive(Debug, Clone, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub struct Container {
    pub name: String,
    #[eure(flatten)]
    pub extra: IndexMap<String, String>,
}

/// BUG: Flatten with IndexMap doesn't mark fields as accessed.
#[test]
fn test_flatten_indexmap_simple() {
    use eure::eure;

    let doc = eure!({
        name = "test"
        foo = "bar"
    });

    let result: Container = doc.parse(doc.get_root_id()).unwrap();

    let mut expected_extra = IndexMap::new();
    expected_extra.insert("foo".to_string(), "bar".to_string());

    assert_eq!(
        result,
        Container {
            name: "test".to_string(),
            extra: expected_extra,
        }
    );
}

// =============================================================================
// Flatten with nested union (blog.eure.dev pattern)
// =============================================================================

#[derive(Debug, Clone, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub enum Item<T> {
    Normal(T),
    List(Vec<T>),
}

#[derive(Debug, Clone, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub enum TextOrNested<T> {
    Text(String),
    Nested(T),
}

#[derive(Debug, Clone, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub struct Level3 {
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub struct Level2 {
    pub header: String,
    #[eure(flatten)]
    pub sections: IndexMap<String, Item<TextOrNested<Level3>>>,
}

/// BUG: Flatten with nested union IndexMap values.
#[test]
fn test_flatten_indexmap_with_nested_union() {
    use eure::eure;

    let doc = eure!({
        header = "Header"
        intro = "Introduction text"
    });

    let result: Level2 = doc.parse(doc.get_root_id()).unwrap();

    let mut expected_sections = IndexMap::new();
    expected_sections.insert(
        "intro".to_string(),
        Item::Normal(TextOrNested::Text("Introduction text".to_string())),
    );

    assert_eq!(
        result,
        Level2 {
            header: "Header".to_string(),
            sections: expected_sections,
        }
    );
}
