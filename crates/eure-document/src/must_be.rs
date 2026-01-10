//! Zero-sized types for compile-time literal matching in `ParseDocument`.
//!
//! This module provides `MustBeText<M>`, a zero-sized type that only successfully
//! parses from a specific Text value (content + language). It's similar to
//! monostate's `MustBe!` macro but for Eure's `ParseDocument` trait.

use core::marker::PhantomData;

use crate::parse::{ParseContext, ParseDocument, ParseError, ParseErrorKind};
use crate::text::{Language, Text};

/// Marker trait for `MustBeText` types.
///
/// Types implementing this trait specify the expected content and language
/// for text literal matching.
pub trait MustBeTextMarker: Copy {
    /// The expected text content.
    const CONTENT: &'static str;
    /// The expected language.
    const LANGUAGE: Language;
}

/// Zero-sized type that only parses from a specific Text value.
///
/// This type implements `ParseDocument` and succeeds only when the parsed
/// value matches the expected content and has a compatible language.
///
/// Use the `MustBeText!` macro to create instances of this type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MustBeText<M: MustBeTextMarker>(PhantomData<M>);

impl<M: MustBeTextMarker> MustBeText<M> {
    /// Create a new instance.
    pub const fn new() -> Self {
        MustBeText(PhantomData)
    }
}

impl<M: MustBeTextMarker> ParseDocument<'_> for MustBeText<M> {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        let text: Text = ctx.parse()?;
        if text.content == M::CONTENT && text.language.is_compatible_with(&M::LANGUAGE) {
            Ok(MustBeText(PhantomData))
        } else {
            Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::LiteralMismatch {
                    expected: alloc::format!("{:?} {:?}", M::LANGUAGE, M::CONTENT),
                    actual: alloc::format!("{:?} {:?}", text.language, text.content),
                },
            })
        }
    }
}
