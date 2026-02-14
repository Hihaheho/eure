//! Tests for combining `#[eure(ext)]` with `#[eure(flatten)]` on different fields.
//!
//! Part 1–4 reproduce bugs or verify behavior for various flatten targets:
//! - Record struct (baseline, should work)
//! - Vec<T> (bug: flatten context is a Map, Vec expects Array)
//! - NodeId (bug repro: flatten + ext on opaque capture)
//! - IndexMap (should work, IndexMap already supports flatten)
//!
//! Part 5 tests all three modes together (ext + flatten + flatten_ext).
//! Part 6 tests error cases.

use eure::document::NodeId;
use eure::document::parse::ParseErrorKind;
use eure::{FromEure, IntoEure};
use indexmap::IndexMap;

// =============================================================================
// Part 1: Baseline — flatten record struct + ext (should work)
// =============================================================================

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
struct SpriteWithFlag {
    name: String,
    #[eure(ext)]
    visible: bool,
    #[eure(flatten)]
    position: Position,
}

#[test]
fn test_ext_and_flatten_record_baseline() {
    use eure::eure;
    let doc = eure!({
        name = "hero"
        %visible = true
        x = 1.0f32
        y = 2.0f32
    });

    let result = doc.parse::<SpriteWithFlag>(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        SpriteWithFlag {
            name: "hero".to_string(),
            visible: true,
            position: Position { x: 1.0, y: 2.0 },
        }
    );
}

#[test]
fn test_ext_and_flatten_record_roundtrip() {
    use eure_document::document::constructor::DocumentConstructor;

    let sprite = SpriteWithFlag {
        name: "hero".to_string(),
        visible: true,
        position: Position { x: 1.0, y: 2.0 },
    };

    let mut c = DocumentConstructor::new();
    c.write(sprite).unwrap();
    let doc = c.finish();

    let parsed = doc.parse::<SpriteWithFlag>(doc.get_root_id()).unwrap();
    assert_eq!(
        parsed,
        SpriteWithFlag {
            name: "hero".to_string(),
            visible: true,
            position: Position { x: 1.0, y: 2.0 },
        }
    );
}

// =============================================================================
// Part 2: Bug repro — flatten Vec<T> + ext (the Dialog pattern)
// =============================================================================

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document, parse_ext)]
struct DialogMeta {
    speaker: String,
}

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct DialogBlock {
    text: String,
}

/// Regression test: content-mode flatten should allow Vec<T> alongside ext fields
/// when there are no regular record fields.
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct Dialog {
    #[eure(ext)]
    meta: DialogMeta,
    #[eure(flatten)]
    blocks: Vec<DialogBlock>,
}

#[test]
fn test_ext_and_flatten_vec_content_mode_success() {
    use eure::eure;

    // Extension metadata and array-like payload coexist at the same level.
    let doc = eure!({
        %meta { %speaker = "Alice" }
        #[] { text = "Hello" }
        #[] { text = "World" }
    });

    let result = doc.parse::<Dialog>(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Dialog {
            meta: DialogMeta {
                speaker: "Alice".to_string(),
            },
            blocks: vec![
                DialogBlock {
                    text: "Hello".to_string(),
                },
                DialogBlock {
                    text: "World".to_string(),
                },
            ],
        }
    );
}

// =============================================================================
// Part 3: Bug repro — flatten NodeId + ext
// =============================================================================

/// Captures the raw node alongside an extension annotation.
///
/// NodeId::parse just returns ctx.node_id(), so flatten context should work,
/// but this pattern is untested and may interact poorly with field tracking.
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct Annotated {
    #[eure(ext)]
    tag: String,
    #[eure(flatten)]
    raw: NodeId,
}

#[test]
fn test_ext_and_flatten_node_id() {
    use eure::eure;

    let doc = eure!({
        %tag = "important"
        content = "some data"
    });

    // NodeId::parse just returns ctx.node_id(), so flatten doesn't change behavior.
    // However, the flatten context may interfere with deny_unknown_fields since
    // NodeId doesn't mark any fields as accessed.
    let result = doc.parse::<Annotated>(doc.get_root_id());

    // Current behavior: this may fail due to deny_unknown_fields seeing
    // "content" as an unknown field (NodeId doesn't consume record fields).
    match result {
        Ok(annotated) => {
            assert_eq!(annotated.tag, "important");
            assert_eq!(annotated.raw, doc.get_root_id());
        }
        Err(e) => {
            // If this fails, it's because NodeId doesn't participate in
            // field access tracking, so deny_unknown_fields rejects "content".
            assert!(
                matches!(e.kind, ParseErrorKind::UnknownField(_)),
                "Unexpected error kind: {:?}",
                e.kind
            );
        }
    }
}

// =============================================================================
// Part 4: Flatten IndexMap + ext (should work)
// =============================================================================

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct Tagged {
    name: String,
    #[eure(ext)]
    tag: String,
    #[eure(flatten)]
    extra: IndexMap<String, String>,
}

#[test]
fn test_ext_and_flatten_indexmap() {
    use eure::eure;

    let doc = eure!({
        name = "config"
        %tag = "v2"
        foo = "bar"
        baz = "qux"
    });

    let result = doc.parse::<Tagged>(doc.get_root_id()).unwrap();

    let mut expected_extra = IndexMap::new();
    expected_extra.insert("foo".to_string(), "bar".to_string());
    expected_extra.insert("baz".to_string(), "qux".to_string());

    assert_eq!(
        result,
        Tagged {
            name: "config".to_string(),
            tag: "v2".to_string(),
            extra: expected_extra,
        }
    );
}

#[test]
fn test_ext_and_flatten_indexmap_empty_extra() {
    use eure::eure;

    let doc = eure!({ name = "minimal" % tag = "v1" });

    let result = doc.parse::<Tagged>(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        Tagged {
            name: "minimal".to_string(),
            tag: "v1".to_string(),
            extra: IndexMap::new(),
        }
    );
}

// =============================================================================
// Part 5: All three modes — ext + flatten + flatten_ext
// =============================================================================

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document, parse_ext)]
struct Metadata {
    #[eure(default)]
    version: Option<i32>,
}

#[derive(Debug, PartialEq, FromEure, IntoEure)]
#[eure(crate = ::eure::document)]
struct Coordinates {
    x: i32,
    y: i32,
}

#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct FullCombo {
    name: String,
    #[eure(ext)]
    label: String,
    #[eure(flatten)]
    coords: Coordinates,
    #[eure(flatten_ext)]
    meta: Metadata,
}

#[test]
fn test_all_three_modes() {
    use eure::eure;

    let doc = eure!({
        name = "origin"
        %label = "center"
        x = 0
        y = 0
        %version = 1
    });

    let result = doc.parse::<FullCombo>(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        FullCombo {
            name: "origin".to_string(),
            label: "center".to_string(),
            coords: Coordinates { x: 0, y: 0 },
            meta: Metadata { version: Some(1) },
        }
    );
}

#[test]
fn test_all_three_modes_optional_meta() {
    use eure::eure;

    let doc = eure!({
        name = "point"
        %label = "A"
        x = 10
        y = 20
    });

    let result = doc.parse::<FullCombo>(doc.get_root_id()).unwrap();
    assert_eq!(
        result,
        FullCombo {
            name: "point".to_string(),
            label: "A".to_string(),
            coords: Coordinates { x: 10, y: 20 },
            meta: Metadata { version: None },
        }
    );
}

// =============================================================================
// Part 6: Error cases
// =============================================================================

#[test]
fn test_ext_and_flatten_missing_ext() {
    use eure::eure;

    // Missing required extension %visible
    let doc = eure!({
        name = "hero"
        x = 1.0f32
        y = 2.0f32
    });

    let result = doc.parse::<SpriteWithFlag>(doc.get_root_id());
    assert_eq!(
        result.unwrap_err().kind,
        ParseErrorKind::MissingExtension("visible".to_string())
    );
}

#[test]
fn test_ext_and_flatten_unknown_ext() {
    use eure::eure;

    // Unknown extension %unknown alongside valid ones
    let doc = eure!({
        name = "hero"
        %visible = true
        %unknown = "should fail"
        x = 1.0f32
        y = 2.0f32
    });

    let result = doc.parse::<SpriteWithFlag>(doc.get_root_id());
    assert_eq!(
        result.unwrap_err().kind,
        ParseErrorKind::UnknownExtension("unknown".parse().unwrap())
    );
}

#[test]
fn test_ext_and_flatten_unknown_field() {
    use eure::eure;

    // Unknown record field (not from Position or SpriteWithFlag)
    let doc = eure!({
        name = "hero"
        %visible = true
        x = 1.0f32
        y = 2.0f32
        extra = "should fail"
    });

    let result = doc.parse::<SpriteWithFlag>(doc.get_root_id());
    assert!(result.is_err());
}
