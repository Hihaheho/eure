use crate::prelude_internal::*;
use crate::value::Map;

/// Data model of a document or a value in a document. Corresponds to the `$data-model` extension.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DataModel {
    /// Serde compatible data model.
    Rust,
    /// JSON compatible data model.
    Json,
    /// Eure full data model including path.
    #[default]
    Eure,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DataModelConfig {
    pub data_model: DataModel,
    pub variant_repr: VariantRepr,
    pub number_key_repr: NumberKeyRepr,
    pub tuple_key_repr: TupleKeyRepr,
    pub boolean_key_repr: BooleanKeyRepr,
    pub tuple_repr: TupleRepr,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// How to represent numeric keys in a data model that does not support numbers as object keys. Corresponds to the `$number-key-repr` extension.
pub enum NumberKeyRepr {
    /// Represent number as string.
    String,
    /// Error on conversion.
    #[default]
    Error,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// How to represent tuple keys in a data model that does not support tuples as object keys. Corresponds to the `$tuple-key-repr` extension.
pub enum TupleKeyRepr {
    /// Represent tuple as string. e.g. "(1,2,3)".
    String,
    /// Error on conversion.
    #[default]
    Error,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// How to represent boolean keys in a data model that does not support booleans as object keys. Corresponds to the `$boolean-key-repr` extension.
pub enum BooleanKeyRepr {
    /// Represent boolean as string. e.g. "true" or "false".
    String,
    /// Error on conversion.
    #[default]
    Error,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// How to represent tuples in a data model that does not support tuples. Corresponds to the `$tuple-repr` extension.
pub enum TupleRepr {
    /// Represent tuple as array. e.g. "[1,2,3]".
    Array,
    /// Represent tuple as number indexed object. e.g. `{0: 1, 1: 2, 2: 3}`. `{"0": 1, "1": 2, "2": 3}` if `NumberKeyRepr` is `String`.
    NumberIndexedObject,
    /// Error on conversion.
    #[default]
    Error,
}

#[derive(Debug, Clone, PartialEq, Default)]
/// How to represent variant in a data model. Corresponds to the `$variant-repr` extension.
pub enum VariantRepr {
    /// Default representation: {"variant-name": {...}}
    #[default]
    External,

    /// Internal tagging: {"type": "variant-name", ...fields...}
    Internal { tag: String },

    /// Adjacent tagging: {"type": "variant-name", "content": {...}}
    Adjacent { tag: String, content: String },

    /// Untagged: just the content without variant information
    Untagged,
}

impl VariantRepr {
    /// Create a VariantRepr from $variant-repr annotation value
    pub fn from_annotation(value: &Value) -> Option<Self> {
        match value {
            Value::Primitive(PrimitiveValue::Text(t)) if t.as_str() == "untagged" => {
                Some(VariantRepr::Untagged)
            }
            Value::Map(Map(map)) => {
                let tag = map
                    .get(&ObjectKey::String("tag".to_string()))
                    .and_then(|v| match v {
                        Value::Primitive(PrimitiveValue::Text(t)) => Some(t.as_str().to_string()),
                        _ => None,
                    });

                let content = map
                    .get(&ObjectKey::String("content".to_string()))
                    .and_then(|v| match v {
                        Value::Primitive(PrimitiveValue::Text(t)) => Some(t.as_str().to_string()),
                        _ => None,
                    });

                match (tag, content) {
                    (Some(tag), Some(content)) => Some(VariantRepr::Adjacent { tag, content }),
                    (Some(tag), None) => Some(VariantRepr::Internal { tag }),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// How to represent text with non-plaintext language in a data model.
///
/// This controls how `Text` values with `Language::Other(lang)` are serialized
/// to formats that don't natively support language-tagged text.
///
/// Corresponds to the `$text-repr` extension (formerly `$code-repr`).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TextRepr {
    /// Markdown code block string.
    /// e.g. "```rust\nfn main() { println!(\"Hello, world!\"); }\n```".
    Markdown,
    /// Content only string, discarding language information.
    /// e.g. "fn main() { println!(\"Hello, world!\"); }".
    String,
    /// Object with language and content fields.
    /// e.g. `{"language": "rust", "content": "fn main() { println!(\"Hello, world!\"); }"}`.
    Object {
        language_key: String,
        content_key: String,
    },
    /// Error on conversion.
    #[default]
    Error,
}
