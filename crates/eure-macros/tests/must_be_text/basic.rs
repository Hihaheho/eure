use eure::document::EureDocument;
use eure::document::must_be::{MustBeText, MustBeTextMarker};
use eure::document::parse::ParseError;
use eure::eure;
use eure_macros::MustBeText;

/// Helper to parse with type inference from the marker value.
fn check_parse<M: MustBeTextMarker>(
    doc: &EureDocument,
    _marker: MustBeText<M>,
) -> Result<MustBeText<M>, ParseError> {
    doc.parse(doc.get_root_id())
}

#[test]
fn test_must_be_text_implicit() {
    let doc = eure!({ = @code("any") });
    let result = check_parse(&doc, MustBeText!("any"));
    assert!(result.is_ok());
}

#[test]
fn test_must_be_text_implicit_mismatch() {
    let doc = eure!({ = @code("other") });
    let result = check_parse(&doc, MustBeText!("any"));
    assert!(result.is_err());
}

#[test]
fn test_must_be_text_plaintext() {
    let doc = eure!({ = "hello" });
    let result = check_parse(&doc, MustBeText!(plaintext, "hello"));
    assert!(result.is_ok());
}

#[test]
fn test_must_be_text_plaintext_mismatch() {
    let doc = eure!({ = "world" });
    let result = check_parse(&doc, MustBeText!(plaintext, "hello"));
    assert!(result.is_err());
}

#[test]
fn test_must_be_text_other_language() {
    let doc = eure!({ = @code("rust", "None") });
    let result = check_parse(&doc, MustBeText!(rust, "None"));
    assert!(result.is_ok());
}

#[test]
fn test_must_be_text_other_language_mismatch() {
    let doc = eure!({ = @code("rust", "Some") });
    let result = check_parse(&doc, MustBeText!(rust, "None"));
    assert!(result.is_err());
}

#[test]
fn test_must_be_text_compatibility_implicit_to_any() {
    // Implicit language is compatible with any expected language
    let doc = eure!({ = @code("any") });
    let result = check_parse(&doc, MustBeText!(rust, "any"));
    assert!(result.is_ok());
}

#[test]
fn test_must_be_text_compatibility_any_to_implicit() {
    // Any language is compatible with Implicit expectation
    let doc = eure!({ = @code("rust", "code") });
    let result = check_parse(&doc, MustBeText!("code"));
    assert!(result.is_ok());
}
