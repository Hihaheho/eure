//! Interop-only schema metadata.
//!
//! These types describe non-native wire representations used when bridging Eure
//! with external ecosystems (JSON/Serde/codegen).

use eure_document::parse::{FromEure, ParseContext, ParseError, ParseErrorKind};

/// How to represent union variants on an external wire format.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum VariantRepr {
    // Default intentionally stays `External` for interop consumers that call
    // `VariantRepr::default()`. Eure union semantics do not use this default;
    // they flow through `UnionInterop { variant_repr: None }`.
    /// External tagging: {"variant-name": {...}}
    #[default]
    External,
    /// Internal tagging: {"type": "variant-name", ...fields...}
    Internal { tag: String },
    /// Adjacent tagging: {"type": "variant-name", "content": {...}}
    Adjacent { tag: String, content: String },
    /// Untagged: infer variant from content shape/value.
    Untagged,
}

/// Interop metadata attached to union schemas.
#[derive(Debug, Clone, PartialEq, Default, eure_macros::FromEure)]
#[eure(crate = eure_document, rename_all = "kebab-case")]
pub struct UnionInterop {
    /// Optional representation hint for external wire formats.
    #[eure(default)]
    pub variant_repr: Option<VariantRepr>,
}

impl FromEure<'_> for VariantRepr {
    type Error = ParseError;

    fn parse(ctx: &ParseContext<'_>) -> Result<Self, Self::Error> {
        if let Ok(value) = ctx.parse::<&str>() {
            return match value {
                "external" => Ok(VariantRepr::External),
                "untagged" => Ok(VariantRepr::Untagged),
                "internal" => Ok(VariantRepr::Internal {
                    tag: "type".to_string(),
                }),
                "adjacent" => Ok(VariantRepr::Adjacent {
                    tag: "type".to_string(),
                    content: "content".to_string(),
                }),
                _ => Err(ParseError {
                    node_id: ctx.node_id(),
                    kind: ParseErrorKind::UnknownVariant(value.to_string()),
                }),
            };
        }

        let rec = ctx.parse_record()?;
        let tag = rec.parse_field_optional::<String>("tag")?;
        let content = rec.parse_field_optional::<String>("content")?;
        rec.allow_unknown_fields()?;

        match (tag, content) {
            (Some(tag), Some(content)) => Ok(VariantRepr::Adjacent { tag, content }),
            (Some(tag), None) => Ok(VariantRepr::Internal { tag }),
            (None, None) => Ok(VariantRepr::External),
            (None, Some(_)) => Err(ParseError {
                node_id: ctx.node_id(),
                kind: ParseErrorKind::MissingField(
                    "tag (required when content is present)".to_string(),
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_repr_default_is_external() {
        assert_eq!(VariantRepr::default(), VariantRepr::External);
    }

    #[test]
    fn union_interop_default_has_no_variant_repr() {
        assert_eq!(UnionInterop::default().variant_repr, None);
    }
}
