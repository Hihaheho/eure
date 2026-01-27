//! Tests for flatten with nested unions.
//!
//! These tests cover scenarios where #[eure(flatten)] is used with map values
//! that are nested union types. This pattern is common in blog.eure.dev's
//! article structure where sections have flattened maps of union-typed content.

use eure::ParseDocument;
use eure::document::parse::ParseErrorKind;

/// When union variant A tries some fields, fails, then variant B succeeds,
/// the accessed field tracking may include A's fields even though A failed.
/// This can cause `deny_unknown_fields()` to incorrectly pass or fail.
#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
enum TestOption {
    /// Tries fields: a, c, e (will fail if e is missing)
    VariantA { a: i32, c: i32, e: i32 },
    /// Tries fields: a, b
    VariantB { a: i32, b: i32 },
}

#[derive(Debug, PartialEq, ParseDocument)]
#[eure(crate = ::eure::document)]
struct FlattenUnionContainer {
    #[eure(flatten)]
    inner: TestOption,
}

/// This test verifies the behavior of accessed field rollback in flatten + union.
///
/// With input { a = 1, b = 2, c = 3, d = 4 }:
/// 1. VariantA tries 'a', 'c', then fails on 'e' (doesn't exist)
/// 2. VariantB tries 'a', 'b' and succeeds
/// 3. Field 'c' was accessed by VariantA but VariantA failed
/// 4. Field 'd' was never accessed by any variant
///
/// The UnionParser has snapshot/rollback logic that SHOULD revert VariantA's
/// accesses when it fails. If working correctly, 'c' should NOT be in the
/// accessed set after VariantB succeeds.
#[test]
fn test_flatten_union_accessed_field_rollback() {
    use eure::eure;

    let doc = eure!({
        a = 1
        b = 2
        c = 3
        d = 4
    });

    let result = doc.parse::<FlattenUnionContainer>(doc.get_root_id());

    // The parsing succeeds with VariantB, but deny_unknown_fields complains
    // about fields not accessed by the successful variant.
    //
    // With proper rollback: 'c' is NOT in accessed set, so it's reported as unknown
    // (This is the CORRECT behavior - 'c' wasn't accessed by VariantB)
    let err = result.unwrap_err();
    assert_eq!(err.kind, ParseErrorKind::UnknownField("c".to_string()));
}

/// Test that demonstrates correct behavior when all accessed fields
/// belong to the successful variant.
#[test]
fn test_flatten_union_no_extra_fields() {
    use eure::eure;

    // Only fields needed by VariantB
    let doc = eure!({
        a = 1
        b = 2
    });

    let result: FlattenUnionContainer = doc.parse(doc.get_root_id()).unwrap();
    assert_eq!(result.inner, TestOption::VariantB { a: 1, b: 2 });
}
