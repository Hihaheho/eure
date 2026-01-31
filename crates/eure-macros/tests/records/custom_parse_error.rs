use eure::document::parse::{ParseContext, ParseError, ParseErrorKind};
use eure::FromEure;

/// A custom inner error type that can be returned from manual FromEure implementations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum InnerError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("validation error: {message}")]
    Validation { message: String },
}

/// A custom error type that combines ParseError and InnerError.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CustomError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("inner error: {0}")]
    Inner(#[from] InnerError),
}

/// An inner struct that uses manual FromEure implementation
/// that returns InnerError.
#[derive(Debug, PartialEq)]
pub struct InnerStruct {
    pub value: i32,
}

impl<'doc> eure::document::parse::FromEure<'doc> for InnerStruct {
    type Error = InnerError;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Self, Self::Error> {
        let rec = ctx.parse_record()?;
        let value: i32 = rec.parse_field("value")?;

        // Demonstrate returning a custom InnerError based on validation
        if value < 0 {
            return Err(InnerError::Validation {
                message: "value must be non-negative".to_string(),
            });
        }

        rec.deny_unknown_fields()?;
        Ok(InnerStruct { value })
    }
}

/// An outer struct that uses the derive macro with custom error type.
/// The derive macro will use CustomError as the error type instead of ParseError.
/// Since CustomError implements From<InnerError>, the `?` operator will convert
/// InnerError to CustomError automatically.
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document, parse_error = CustomError)]
pub struct OuterStruct {
    pub name: String,
    pub inner: InnerStruct,
}

#[test]
fn test_custom_error_success() {
    use eure::eure;
    let doc = eure!({
        name = "test"
        inner { value = 42 }
    });
    let result = doc.parse::<OuterStruct>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        OuterStruct {
            name: "test".to_string(),
            inner: InnerStruct { value: 42 }
        }
    );
}

#[test]
fn test_custom_error_from_parse_error() {
    use eure::eure;
    // Missing required field "name"
    let doc = eure!({
        inner { value = 42 }
    });
    let result = doc.parse::<OuterStruct>(doc.get_root_id());
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Error should be converted from ParseError
    match err {
        CustomError::Parse(parse_err) => {
            assert!(matches!(parse_err.kind, ParseErrorKind::MissingField(_)));
        }
        _ => panic!("expected CustomError::Parse, got {:?}", err),
    }
}

#[test]
fn test_custom_error_from_inner_error() {
    use eure::eure;
    // InnerStruct validation fails (negative value)
    let doc = eure!({
        name = "test"
        inner { value = -1 }
    });
    let result = doc.parse::<OuterStruct>(doc.get_root_id());
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Error should be converted from InnerError via From<InnerError>
    match err {
        CustomError::Inner(InnerError::Validation { message }) => {
            assert_eq!(message, "value must be non-negative");
        }
        _ => panic!("expected CustomError::Inner(InnerError::Validation), got {:?}", err),
    }
}

/// A nested struct that contains another derived struct
/// to test that nested types also work with custom errors.
#[derive(Debug, PartialEq, FromEure)]
#[eure(crate = ::eure::document, parse_error = CustomError)]
pub struct NestedStruct {
    pub outer: OuterStruct,
}

#[test]
fn test_nested_custom_error() {
    use eure::eure;
    let doc = eure!({
        outer {
            name = "test"
            inner { value = 10 }
        }
    });
    let result = doc.parse::<NestedStruct>(doc.get_root_id());
    assert_eq!(
        result.unwrap(),
        NestedStruct {
            outer: OuterStruct {
                name: "test".to_string(),
                inner: InnerStruct { value: 10 }
            }
        }
    );
}

#[test]
fn test_nested_inner_error_bubbles_up() {
    use eure::eure;
    let doc = eure!({
        outer {
            name = "test"
            inner { value = -5 }
        }
    });
    let result = doc.parse::<NestedStruct>(doc.get_root_id());
    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        CustomError::Inner(InnerError::Validation { message }) => {
            assert_eq!(message, "value must be non-negative");
        }
        _ => panic!("expected CustomError::Inner(InnerError::Validation), got {:?}", err),
    }
}
