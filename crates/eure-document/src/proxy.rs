//! Proxy types for use with `#[eure(via = "...")]` attribute.

extern crate alloc;

use alloc::borrow::{Cow, ToOwned};

use crate::document::constructor::DocumentConstructor;
use crate::parse::{FromEure, ParseContext, ParseError};
use crate::write::{IntoEure, WriteError};

/// A proxy type that enables borrowing from the document when parsing `Cow<'doc, T>`.
///
/// Unlike the default `Cow<'static, T>` implementation which always returns `Cow::Owned`,
/// this proxy returns `Cow::Borrowed` by parsing `&'doc T` directly from the document.
///
/// # Example
/// ```ignore
/// use eure_macros::FromEure;
/// use std::borrow::Cow;
///
/// #[derive(FromEure)]
/// struct Data<'doc> {
///     #[eure(via = "eure_document::proxy::BorrowedCow")]
///     name: Cow<'doc, str>,
/// }
/// ```
pub struct BorrowedCow;

impl<'doc, T> FromEure<'doc, Cow<'doc, T>> for BorrowedCow
where
    T: ToOwned + ?Sized,
    for<'a> &'a T: FromEure<'a>,
    for<'a> <&'a T as FromEure<'a>>::Error: Into<ParseError>,
{
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'doc>) -> Result<Cow<'doc, T>, Self::Error> {
        ctx.parse::<&'doc T>()
            .map(Cow::Borrowed)
            .map_err(Into::into)
    }
}

impl<'a, T> IntoEure<Cow<'a, T>> for BorrowedCow
where
    T: ToOwned + ?Sized,
    T::Owned: IntoEure,
{
    fn write(value: Cow<'a, T>, c: &mut DocumentConstructor) -> Result<(), WriteError> {
        <T::Owned as IntoEure>::write(value.into_owned(), c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::node::NodeValue;
    use crate::eure;
    use crate::text::Text;
    use crate::value::PrimitiveValue;

    #[test]
    fn test_borrowed_cow_str_from_eure() {
        let doc = eure!({ name = "hello" });
        let root_id = doc.get_root_id();
        let rec = doc.parse_record(root_id).unwrap();
        let value: Cow<'_, str> = rec.parse_field_with("name", BorrowedCow::parse).unwrap();
        assert!(matches!(value, Cow::Borrowed(_)));
        assert_eq!(value, "hello");
    }

    #[test]
    fn test_borrowed_cow_str_into_eure() {
        let mut c = DocumentConstructor::new();
        let value: Cow<'_, str> = Cow::Borrowed("hello");
        BorrowedCow::write(value, &mut c).unwrap();
        let doc = c.finish();
        assert_eq!(
            doc.root().content,
            NodeValue::Primitive(PrimitiveValue::Text(Text::plaintext("hello")))
        );
    }
}
