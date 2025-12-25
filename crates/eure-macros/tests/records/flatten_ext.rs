//! Tests for #[eure(flatten_ext)] in parse_ext context
//!
//! When using #[eure(parse_ext)]:
//! - #[eure(flatten)] is NOT allowed (compile error) because it targets record scope
//! - #[eure(flatten_ext)] flattens extension types, sharing extension access tracking
//!
//! In regular (non-parse_ext) context:
//! - #[eure(flatten)] flattens record types
//! - #[eure(flatten_ext)] flattens extension types from record context

use eure::ParseDocument;
use eure::document::parse::ParseErrorKind;
use std::collections::HashMap;

// =============================================================================
// Case 1: Both parent and child have parse_ext - use flatten_ext
// =============================================================================

/// Child that also parses from extensions
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, parse_ext)]
struct ExtValidation {
    min: Option<i32>,
    max: Option<i32>,
}

/// Both parse from extensions - use flatten_ext for extension flattening
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, parse_ext)]
struct FullExtMeta {
    optional: bool,
    #[eure(flatten_ext)]
    validation: ExtValidation,
}

// =============================================================================
// Case 2: Catch all unknown extensions with flatten_ext
// =============================================================================

/// Catch all unknown extensions using flatten_ext
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, parse_ext)]
struct UnknownExtMeta {
    a: i32,
    b: i32,
    #[eure(flatten_ext)]
    unknown: HashMap<String, String>,
}

// =============================================================================
// Case 3: Record context with standalone #[eure(ext)] attribute
// =============================================================================

/// Record that parses a single extension field
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct RecordWithSingleExt {
    name: String,
    #[eure(ext)]
    deprecated: bool,
}

/// Record with multiple extension fields
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct RecordWithMultipleExt {
    name: String,
    #[eure(ext)]
    version: i32,
    #[eure(ext)]
    deprecated: bool,
}

// =============================================================================
// Case 4: Record context can use both flatten and flatten_ext
// =============================================================================

/// Nested record type
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct NestedRecord {
    x: i32,
    y: i32,
}

/// Nested extension type
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, parse_ext)]
struct NestedExt {
    meta: Option<String>,
}

/// Record context using flatten for record types
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct RecordWithFlatten {
    name: String,
    #[eure(flatten)]
    nested: NestedRecord,
}

/// Record context using flatten_ext for extension types
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct RecordWithFlattenExt {
    name: String,
    #[eure(flatten_ext)]
    ext: NestedExt,
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
pub struct WronglyRecord {
    a: i32,
    b: i32,
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct FlattenExtWithWrongRecord {
    #[eure(flatten_ext)]
    ext: WronglyRecord,
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document, parse_ext)]
struct WronglyExt {
    c: i32,
    d: i32,
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct FlattenWithWrongExt {
    #[eure(flatten)]
    ext: WronglyExt,
}

// =============================================================================
// Tests
// =============================================================================

#[test]
fn test_flatten_ext_both_parse_ext() {
    use eure::eure;
    let doc = eure!({
        %optional = true,
        %min = 0,
        %max = 100
    });

    let result = doc.parse::<FullExtMeta>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        FullExtMeta {
            optional: true,
            validation: ExtValidation {
                min: Some(0),
                max: Some(100),
            }
        }
    );
}

#[test]
fn test_flatten_ext_unknown_extension_detection() {
    use eure::eure;
    let doc = eure!({
        %optional = true,
        %min = 0,
        %max = 100,
        %unknown = "extra"
    });

    // With flatten_ext, unknown extensions should be detected by root parser
    // Currently uses allow_unknown_extensions()
    let result = doc.parse::<FullExtMeta>(doc.get_root_id());
    assert!(result.is_ok()); // Current behavior allows unknown
}

#[test]
fn test_catch_all_unknown_extensions() {
    use eure::eure;
    let doc = eure!({
        %a = 1
        %b = 2
        %c = "a"
        %d = "b"
    });

    let result = doc.parse::<UnknownExtMeta>(doc.get_root_id());

    assert_eq!(
        result.unwrap(),
        UnknownExtMeta {
            a: 1,
            b: 2,
            unknown: HashMap::from([
                ("c".to_string(), "a".to_string()),
                ("d".to_string(), "b".to_string())
            ]),
        }
    );
}

#[test]
fn test_record_with_flatten() {
    use eure::eure;
    let doc = eure!({
        name = "test",
        x = 10,
        y = 20
    });

    let result = doc.parse::<RecordWithFlatten>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        RecordWithFlatten {
            name: "test".to_string(),
            nested: NestedRecord { x: 10, y: 20 },
        }
    );
}

#[test]
fn test_record_with_flatten_ext() {
    use eure::eure;
    let doc = eure!({
        name = "test",
        %meta = "some metadata"
    });

    let result = doc.parse::<RecordWithFlattenExt>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        RecordWithFlattenExt {
            name: "test".to_string(),
            ext: NestedExt {
                meta: Some("some metadata".to_string()),
            },
        }
    );
}

#[test]
fn test_record_with_single_ext() {
    use eure::eure;
    let doc = eure!({
        name = "test",
        %deprecated = true
    });

    let result = doc.parse::<RecordWithSingleExt>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        RecordWithSingleExt {
            name: "test".to_string(),
            deprecated: true,
        }
    );
}

#[test]
fn test_record_with_single_ext_rejects_unknown() {
    use eure::eure;
    let doc = eure!({
        name = "test",
        %deprecated = true,
        %unknown = "should fail"
    });

    let result = doc.parse::<RecordWithSingleExt>(doc.get_root_id());
    assert_eq!(
        result.unwrap_err().kind,
        ParseErrorKind::UnknownExtension("unknown".parse().unwrap())
    );
}

#[test]
fn test_record_with_multiple_ext() {
    use eure::eure;
    let doc = eure!({
        name = "test",
        %version = 42,
        %deprecated = false
    });

    let result = doc.parse::<RecordWithMultipleExt>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        RecordWithMultipleExt {
            name: "test".to_string(),
            version: 42,
            deprecated: false,
        }
    );
}

#[test]
fn test_record_with_multiple_ext_missing_required() {
    use eure::eure;
    let doc = eure!({
        name = "test",
        %version = 42
        // missing %deprecated
    });

    let result = doc.parse::<RecordWithMultipleExt>(doc.get_root_id());
    assert_eq!(
        result.unwrap_err().kind,
        ParseErrorKind::MissingExtension("deprecated".to_string())
    );
}

#[test]
fn test_flatten_ext_with_wrong_record_type() {
    use eure::eure;
    // FlattenExtWithWrongRecord uses #[eure(flatten_ext)] but WronglyRecord
    // parses as a record (calls parse_record()), not extensions.
    // This is a user error that should be caught.
    let doc = eure!({
        a = 1,
        b = 2
    });

    let result = doc.parse::<FlattenExtWithWrongRecord>(doc.get_root_id());
    assert_eq!(
        result.unwrap_err().kind,
        ParseErrorKind::RecordInExtensionScope
    );
}

#[test]
fn test_flatten_with_ext_parsing_type() {
    use eure::eure;
    // FlattenWithWrongExt uses #[eure(flatten)] with a type that has #[eure(parse_ext)].
    // This is allowed because extensions can be parsed from any context.
    // The child type parses extensions while parent tracks record fields.
    let doc = eure!({
        %c = 1,
        %d = 2
    });

    let result = doc.parse::<FlattenWithWrongExt>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        FlattenWithWrongExt {
            ext: WronglyExt { c: 1, d: 2 }
        }
    );
}
