use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt;

use eure::document::EureDocument;
use eure::document::constructor::DocumentConstructor;
use eure::document::identifier::Identifier;
use eure::document::node::NodeValue;
use eure::document::parse::VariantPath;
use eure::document::path::PathSegment;
use eure::value::{ObjectKey, PrimitiveValue, Text, Tuple};
use eure_schema::interop::VariantRepr;
use eure_schema::{
    SchemaDocument, SchemaNodeContent, SchemaNodeId, UnionSchema, UnknownFieldsPolicy,
};
use num_bigint::BigInt;
use serde::Deserialize;
use serde::de::Error as _;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};

use crate::error::DeError;

#[derive(Debug, Clone)]
enum EureContent<'de> {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    F32(f32),
    F64(f64),
    Char(char),
    String(Cow<'de, str>),
    Bytes(Cow<'de, [u8]>),
    Unit,
    Seq(Vec<EureContent<'de>>),
    Map(Vec<(EureContent<'de>, EureContent<'de>)>),
}

impl<'de> Deserialize<'de> for EureContent<'de> {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        de.deserialize_any(EureContentVisitor)
    }
}

impl<'de> serde::Deserializer<'de> for EureContent<'de> {
    type Error = DeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Bool(value) => visitor.visit_bool(value),
            Self::I8(value) => visitor.visit_i8(value),
            Self::I16(value) => visitor.visit_i16(value),
            Self::I32(value) => visitor.visit_i32(value),
            Self::I64(value) => visitor.visit_i64(value),
            Self::I128(value) => visitor.visit_i128(value),
            Self::U8(value) => visitor.visit_u8(value),
            Self::U16(value) => visitor.visit_u16(value),
            Self::U32(value) => visitor.visit_u32(value),
            Self::U64(value) => visitor.visit_u64(value),
            Self::U128(value) => visitor.visit_u128(value),
            Self::F32(value) => visitor.visit_f32(value),
            Self::F64(value) => visitor.visit_f64(value),
            Self::Char(value) => visitor.visit_char(value),
            Self::String(Cow::Borrowed(value)) => visitor.visit_borrowed_str(value),
            Self::String(Cow::Owned(value)) => visitor.visit_string(value),
            Self::Bytes(Cow::Borrowed(value)) => visitor.visit_borrowed_bytes(value),
            Self::Bytes(Cow::Owned(value)) => visitor.visit_byte_buf(value),
            Self::Unit => visitor.visit_unit(),
            Self::Seq(values) => visitor.visit_seq(EureContentSeqAccess {
                iter: values.into_iter(),
            }),
            Self::Map(entries) => visitor.visit_map(EureContentMapAccess {
                iter: entries.into_iter(),
                pending_value: None,
            }),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Bool(value) => visitor.visit_bool(value),
            other => Err(type_mismatch("boolean", eure_content_type(&other))),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::I8(value) => visitor.visit_i8(value),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::I16(value) => visitor.visit_i16(value),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::I32(value) => visitor.visit_i32(value),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Accept any integer variant — the visitor handles all widths.
        match self {
            Self::I8(v) => visitor.visit_i8(v),
            Self::I16(v) => visitor.visit_i16(v),
            Self::I32(v) => visitor.visit_i32(v),
            Self::I64(v) => visitor.visit_i64(v),
            Self::I128(v) => visitor.visit_i128(v),
            Self::U8(v) => visitor.visit_u8(v),
            Self::U16(v) => visitor.visit_u16(v),
            Self::U32(v) => visitor.visit_u32(v),
            Self::U64(v) => visitor.visit_u64(v),
            Self::U128(v) => visitor.visit_u128(v),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::I128(value) => visitor.visit_i128(value),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::U8(value) => visitor.visit_u8(value),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::U16(value) => visitor.visit_u16(value),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::U32(value) => visitor.visit_u32(value),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::U64(value) => visitor.visit_u64(value),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::U128(value) => visitor.visit_u128(value),
            other => Err(type_mismatch("integer", eure_content_type(&other))),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::F32(value) => visitor.visit_f32(value),
            other => Err(type_mismatch("float", eure_content_type(&other))),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::F32(v) => visitor.visit_f32(v),
            Self::F64(v) => visitor.visit_f64(v),
            // Accept integer types via coercion (some formats send ints for float fields).
            Self::I8(v) => visitor.visit_f64(v as f64),
            Self::I16(v) => visitor.visit_f64(v as f64),
            Self::I32(v) => visitor.visit_f64(v as f64),
            Self::I64(v) => visitor.visit_f64(v as f64),
            Self::I128(v) => visitor.visit_f64(v as f64),
            Self::U8(v) => visitor.visit_f64(v as f64),
            Self::U16(v) => visitor.visit_f64(v as f64),
            Self::U32(v) => visitor.visit_f64(v as f64),
            Self::U64(v) => visitor.visit_f64(v as f64),
            Self::U128(v) => visitor.visit_f64(v as f64),
            other => Err(type_mismatch("float", eure_content_type(&other))),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Char(value) => visitor.visit_char(value),
            other => Err(type_mismatch("text", eure_content_type(&other))),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::String(Cow::Borrowed(value)) => visitor.visit_borrowed_str(value),
            Self::String(Cow::Owned(value)) => visitor.visit_string(value),
            other => Err(type_mismatch("text", eure_content_type(&other))),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Bytes(Cow::Borrowed(value)) => visitor.visit_borrowed_bytes(value),
            Self::Bytes(Cow::Owned(value)) => visitor.visit_byte_buf(value),
            other => Err(type_mismatch("bytes", eure_content_type(&other))),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Unit => visitor.visit_none(),
            other => visitor.visit_some(other),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Unit => visitor.visit_unit(),
            other => Err(type_mismatch("null", eure_content_type(&other))),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Seq(values) => visitor.visit_seq(EureContentSeqAccess {
                iter: values.into_iter(),
            }),
            other => Err(type_mismatch("array", eure_content_type(&other))),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::Map(entries) => visitor.visit_map(EureContentMapAccess {
                iter: entries.into_iter(),
                pending_value: None,
            }),
            other => Err(type_mismatch("map", eure_content_type(&other))),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(DeError::Custom(
            "enum deserialization is unsupported for EureContent".to_string(),
        ))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Self::String(Cow::Borrowed(value)) => visitor.visit_borrowed_str(value),
            Self::String(Cow::Owned(value)) => visitor.visit_string(value),
            Self::Char(value) => visitor.visit_string(value.to_string()),
            other => Err(type_mismatch("text", eure_content_type(&other))),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

impl<'de> IntoDeserializer<'de, DeError> for EureContent<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

struct EureContentVisitor;

impl<'de> Visitor<'de> for EureContentVisitor {
    type Value = EureContent<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any serde value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(EureContent::Bool(value))
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E> {
        Ok(EureContent::I8(value))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E> {
        Ok(EureContent::I16(value))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E> {
        Ok(EureContent::I32(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(EureContent::I64(value))
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E> {
        Ok(EureContent::I128(value))
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E> {
        Ok(EureContent::U8(value))
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E> {
        Ok(EureContent::U16(value))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E> {
        Ok(EureContent::U32(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(EureContent::U64(value))
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E> {
        Ok(EureContent::U128(value))
    }

    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E> {
        Ok(EureContent::F32(value))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(EureContent::F64(value))
    }

    fn visit_char<E>(self, value: char) -> Result<Self::Value, E> {
        Ok(EureContent::Char(value))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(EureContent::String(Cow::Borrowed(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(EureContent::String(Cow::Owned(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(EureContent::String(Cow::Owned(value)))
    }

    fn visit_borrowed_bytes<E>(self, value: &'de [u8]) -> Result<Self::Value, E> {
        Ok(EureContent::Bytes(Cow::Borrowed(value)))
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(EureContent::Bytes(Cow::Owned(value.to_vec())))
    }

    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        Ok(EureContent::Bytes(Cow::Owned(value)))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(EureContent::Unit)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(EureContent::Unit)
    }

    fn visit_some<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        EureContent::deserialize(de)
    }

    fn visit_newtype_struct<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        EureContent::deserialize(de)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element::<EureContent<'de>>()? {
            values.push(value);
        }
        Ok(EureContent::Seq(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = Vec::new();
        while let Some((key, value)) = map.next_entry::<EureContent<'de>, EureContent<'de>>()? {
            entries.push((key, value));
        }
        Ok(EureContent::Map(entries))
    }
}

struct EureContentMapVisitor;

impl<'de> Visitor<'de> for EureContentMapVisitor {
    type Value = EureContent<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = Vec::new();
        // Use EureContent for keys to support non-string key types (integer, bool, etc.)
        // from formats like MessagePack/YAML. Tag lookup uses content_key_matches which
        // handles EureContent::String comparison correctly.
        while let Some((key, value)) = map.next_entry::<EureContent<'de>, EureContent<'de>>()? {
            entries.push((key, value));
        }
        Ok(EureContent::Map(entries))
    }
}

struct EureContentSeqAccess<'de> {
    iter: std::vec::IntoIter<EureContent<'de>>,
}

impl<'de> SeqAccess<'de> for EureContentSeqAccess<'de> {
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct EureContentMapAccess<'de> {
    iter: std::vec::IntoIter<(EureContent<'de>, EureContent<'de>)>,
    pending_value: Option<EureContent<'de>>,
}

impl<'de> MapAccess<'de> for EureContentMapAccess<'de> {
    type Error = DeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.pending_value = Some(value);
                seed.deserialize(key).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self.pending_value.take().ok_or_else(|| {
            DeError::Custom("map value requested before reading a key".to_string())
        })?;
        seed.deserialize(value)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len() + usize::from(self.pending_value.is_some()))
    }
}

pub struct EureDocumentSeed<'s> {
    pub schema: &'s SchemaDocument,
    pub schema_node_id: SchemaNodeId,
    pub constructor: &'s mut DocumentConstructor,
}

impl<'de, 's> DeserializeSeed<'de> for EureDocumentSeed<'s> {
    type Value = ();

    fn deserialize<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let schema_node_id =
            resolve_schema_node_id(self.schema, self.schema_node_id).map_err(D::Error::custom)?;

        match &self.schema.node(schema_node_id).content {
            SchemaNodeContent::Any => de.deserialize_any(AnyVisitor {
                constructor: self.constructor,
            }),
            SchemaNodeContent::Text(_) => de.deserialize_str(TextVisitor {
                constructor: self.constructor,
            }),
            SchemaNodeContent::Integer(_) => de.deserialize_i64(IntegerVisitor {
                constructor: self.constructor,
            }),
            SchemaNodeContent::Float(_) => de.deserialize_f64(FloatVisitor {
                constructor: self.constructor,
            }),
            SchemaNodeContent::Boolean => de.deserialize_bool(BoolVisitor {
                constructor: self.constructor,
            }),
            SchemaNodeContent::Null => de.deserialize_unit(NullVisitor {
                constructor: self.constructor,
            }),
            SchemaNodeContent::Array(s) => de.deserialize_seq(SeqVisitor {
                schema: self.schema,
                item_schema_id: s.item,
                constructor: self.constructor,
            }),
            SchemaNodeContent::Tuple(s) => de.deserialize_seq(TupleVisitor {
                schema: self.schema,
                elements: &s.elements,
                constructor: self.constructor,
            }),
            SchemaNodeContent::Map(s) => de.deserialize_map(MapVisitor {
                schema: self.schema,
                key_schema_id: s.key,
                value_schema_id: s.value,
                constructor: self.constructor,
            }),
            SchemaNodeContent::Record(s) => de.deserialize_map(RecordVisitor {
                schema: self.schema,
                record_schema: s,
                constructor: self.constructor,
            }),
            SchemaNodeContent::Union(u) => {
                deserialize_union(de, self.schema, u, schema_node_id, self.constructor)
            }
            // Literal: accept any value and write it as-is. serde-eure is a bridge,
            // not a validator. Callers that need literal enforcement should run the
            // schema validator on the resulting EureDocument afterward.
            SchemaNodeContent::Literal(_) => de.deserialize_any(AnyVisitor {
                constructor: self.constructor,
            }),
            SchemaNodeContent::Reference(_) => unreachable!("references are resolved above"),
        }
    }
}

pub fn from_deserializer<'de, D: serde::Deserializer<'de>>(
    de: D,
    schema: &SchemaDocument,
) -> Result<EureDocument, DeError> {
    let mut constructor = DocumentConstructor::new();
    EureDocumentSeed {
        schema,
        schema_node_id: schema.root,
        constructor: &mut constructor,
    }
    .deserialize(de)
    .map_err(DeError::custom)?;
    Ok(constructor.finish())
}

struct AnySeed<'s> {
    constructor: &'s mut DocumentConstructor,
}

impl<'de> DeserializeSeed<'de> for AnySeed<'_> {
    type Value = ();

    fn deserialize<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        de.deserialize_any(AnyVisitor {
            constructor: self.constructor,
        })
    }
}

struct AnyObjectKeySeed;

impl<'de> DeserializeSeed<'de> for AnyObjectKeySeed {
    type Value = ObjectKey;

    fn deserialize<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        de.deserialize_any(AnyObjectKeyVisitor)
    }
}

struct ObjectKeySeed<'s> {
    schema: &'s SchemaDocument,
    schema_node_id: SchemaNodeId,
}

impl<'de> DeserializeSeed<'de> for ObjectKeySeed<'_> {
    type Value = ObjectKey;

    fn deserialize<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let schema_node_id =
            resolve_schema_node_id(self.schema, self.schema_node_id).map_err(D::Error::custom)?;

        match &self.schema.node(schema_node_id).content {
            SchemaNodeContent::Any => de.deserialize_any(AnyObjectKeyVisitor),
            SchemaNodeContent::Text(_) => de.deserialize_any(TextObjectKeyVisitor),
            SchemaNodeContent::Integer(_) => de.deserialize_any(IntegerObjectKeyVisitor),
            SchemaNodeContent::Boolean => de.deserialize_any(BoolObjectKeyVisitor),
            SchemaNodeContent::Literal(expected) => {
                de.deserialize_any(LiteralObjectKeyVisitor { expected })
            }
            SchemaNodeContent::Tuple(tuple_schema) => de.deserialize_seq(TupleObjectKeyVisitor {
                schema: self.schema,
                elements: &tuple_schema.elements,
            }),
            SchemaNodeContent::Union(union_schema) => de.deserialize_any(UnionObjectKeyVisitor {
                schema: self.schema,
                union_schema,
            }),
            SchemaNodeContent::Reference(_) => unreachable!("references are resolved above"),
            SchemaNodeContent::Float(_)
            | SchemaNodeContent::Null
            | SchemaNodeContent::Array(_)
            | SchemaNodeContent::Map(_)
            | SchemaNodeContent::Record(_) => {
                Err(D::Error::custom(unsupported_complex_map_keys().to_string()))
            }
        }
    }
}

struct ArrayElementSeed<'s> {
    schema: &'s SchemaDocument,
    item_schema_id: SchemaNodeId,
    constructor: &'s mut DocumentConstructor,
}

impl<'de> DeserializeSeed<'de> for ArrayElementSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::ArrayIndex(None))
            .map_err(D::Error::custom)?;
        EureDocumentSeed {
            schema: self.schema,
            schema_node_id: self.item_schema_id,
            constructor: self.constructor,
        }
        .deserialize(de)?;
        self.constructor
            .end_scope(scope)
            .map_err(D::Error::custom)?;
        Ok(())
    }
}

struct AnyArrayElementSeed<'s> {
    constructor: &'s mut DocumentConstructor,
}

impl<'de> DeserializeSeed<'de> for AnyArrayElementSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::ArrayIndex(None))
            .map_err(D::Error::custom)?;
        AnySeed {
            constructor: self.constructor,
        }
        .deserialize(de)?;
        self.constructor
            .end_scope(scope)
            .map_err(D::Error::custom)?;
        Ok(())
    }
}

struct TupleElementSeed<'s> {
    schema: &'s SchemaDocument,
    element_schema_id: SchemaNodeId,
    tuple_index: u8,
    constructor: &'s mut DocumentConstructor,
}

impl<'de> DeserializeSeed<'de> for TupleElementSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let scope = self.constructor.begin_scope();
        self.constructor
            .navigate(PathSegment::TupleIndex(self.tuple_index))
            .map_err(D::Error::custom)?;
        EureDocumentSeed {
            schema: self.schema,
            schema_node_id: self.element_schema_id,
            constructor: self.constructor,
        }
        .deserialize(de)?;
        self.constructor
            .end_scope(scope)
            .map_err(D::Error::custom)?;
        Ok(())
    }
}

struct AnyVisitor<'s> {
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for AnyVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any Eure-compatible value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_bool_value(self.constructor, value).map_err(E::custom)
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_integer_value(self.constructor, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_integer_value(self.constructor, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_integer_value(self.constructor, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_integer_value(self.constructor, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_f32_value(self.constructor, value).map_err(E::custom)
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_f64_value(self.constructor, value).map_err(E::custom)
    }

    fn visit_char<E>(self, value: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_string()).map_err(E::custom)
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_owned()).map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_owned()).map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value).map_err(E::custom)
    }

    fn visit_borrowed_bytes<E>(self, value: &'de [u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_bytes_as_array(self.constructor, value).map_err(E::custom)
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_bytes_as_array(self.constructor, value).map_err(E::custom)
    }

    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_bytes_as_array(self.constructor, &value).map_err(E::custom)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_null_value(self.constructor).map_err(E::custom)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_null_value(self.constructor).map_err(E::custom)
    }

    fn visit_some<D>(self, de: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        AnySeed {
            constructor: self.constructor,
        }
        .deserialize(de)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        self.constructor
            .bind_empty_array()
            .map_err(A::Error::custom)?;
        while seq
            .next_element_seed(AnyArrayElementSeed {
                constructor: self.constructor,
            })?
            .is_some()
        {}
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        self.constructor
            .bind_empty_map()
            .map_err(A::Error::custom)?;
        while let Some(key) = map.next_key_seed(AnyObjectKeySeed)? {
            let scope = self.constructor.begin_scope();
            self.constructor
                .navigate(PathSegment::Value(key))
                .map_err(A::Error::custom)?;
            map.next_value_seed(AnySeed {
                constructor: self.constructor,
            })?;
            self.constructor
                .end_scope(scope)
                .map_err(A::Error::custom)?;
        }
        Ok(())
    }
}

struct TextVisitor<'s> {
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for TextVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("text")
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_owned()).map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_owned()).map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value).map_err(E::custom)
    }

    fn visit_char<E>(self, value: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_string()).map_err(E::custom)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_string()).map_err(E::custom)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_string()).map_err(E::custom)
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_string()).map_err(E::custom)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_text_value(self.constructor, value.to_string()).map_err(E::custom)
    }
}

struct IntegerVisitor<'s> {
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for IntegerVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("integer")
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_integer_value(self.constructor, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_integer_value(self.constructor, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_integer_value(self.constructor, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_integer_value(self.constructor, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = parse_bigint(value).map_err(E::custom)?;
        bind_integer_value(self.constructor, value).map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = parse_bigint(value).map_err(E::custom)?;
        bind_integer_value(self.constructor, value).map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = parse_bigint(&value).map_err(E::custom)?;
        bind_integer_value(self.constructor, value).map_err(E::custom)
    }
}

struct FloatVisitor<'s> {
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for FloatVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("float")
    }

    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_f32_value(self.constructor, value).map_err(E::custom)
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_f64_value(self.constructor, value).map_err(E::custom)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_f64_value(self.constructor, value as f64).map_err(E::custom)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_f64_value(self.constructor, value as f64).map_err(E::custom)
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_f64_value(self.constructor, value as f64).map_err(E::custom)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_f64_value(self.constructor, value as f64).map_err(E::custom)
    }
}

struct BoolVisitor<'s> {
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for BoolVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("boolean")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_bool_value(self.constructor, value).map_err(E::custom)
    }
}

struct NullVisitor<'s> {
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for NullVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("null")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_null_value(self.constructor).map_err(E::custom)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        bind_null_value(self.constructor).map_err(E::custom)
    }
}

struct SeqVisitor<'s> {
    schema: &'s SchemaDocument,
    item_schema_id: SchemaNodeId,
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for SeqVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("array")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        self.constructor
            .bind_empty_array()
            .map_err(A::Error::custom)?;
        while seq
            .next_element_seed(ArrayElementSeed {
                schema: self.schema,
                item_schema_id: self.item_schema_id,
                constructor: self.constructor,
            })?
            .is_some()
        {}
        Ok(())
    }
}

struct TupleVisitor<'s> {
    schema: &'s SchemaDocument,
    elements: &'s [SchemaNodeId],
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for TupleVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("tuple")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        self.constructor
            .bind_empty_tuple()
            .map_err(A::Error::custom)?;

        for (index, &element_schema_id) in self.elements.iter().enumerate() {
            let tuple_index = u8::try_from(index)
                .map_err(|_| A::Error::custom("tuple index out of range for Eure tuple"))?;
            let Some(()) = seq.next_element_seed(TupleElementSeed {
                schema: self.schema,
                element_schema_id,
                tuple_index,
                constructor: self.constructor,
            })?
            else {
                return Err(A::Error::custom(DeError::EndOfSequence.to_string()));
            };
        }

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(A::Error::custom(format!(
                "tuple length mismatch: expected {}, got more elements",
                self.elements.len()
            )));
        }

        Ok(())
    }
}

struct MapVisitor<'s> {
    schema: &'s SchemaDocument,
    key_schema_id: SchemaNodeId,
    value_schema_id: SchemaNodeId,
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for MapVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        self.constructor
            .bind_empty_map()
            .map_err(A::Error::custom)?;

        while let Some(key) = map.next_key_seed(ObjectKeySeed {
            schema: self.schema,
            schema_node_id: self.key_schema_id,
        })? {
            let scope = self.constructor.begin_scope();
            self.constructor
                .navigate(PathSegment::Value(key))
                .map_err(A::Error::custom)?;
            map.next_value_seed(EureDocumentSeed {
                schema: self.schema,
                schema_node_id: self.value_schema_id,
                constructor: self.constructor,
            })?;
            self.constructor
                .end_scope(scope)
                .map_err(A::Error::custom)?;
        }

        Ok(())
    }
}

/// Recursively collect `(field_name → (SchemaNodeId, optional))` from a flattened schema.
/// Only `Record` schemas contribute fields; other node types are silently ignored.
fn collect_flatten_fields(
    schema: &SchemaDocument,
    node_id: SchemaNodeId,
    out: &mut std::collections::HashMap<String, (SchemaNodeId, bool)>,
) {
    let Ok(resolved) = resolve_schema_node_id(schema, node_id) else {
        return;
    };

    match &schema.node(resolved).content {
        SchemaNodeContent::Record(rec) => {
            for (name, field) in &rec.properties {
                out.entry(name.clone())
                    .or_insert((field.schema, field.optional));
            }
            for &inner in &rec.flatten {
                collect_flatten_fields(schema, inner, out);
            }
        }
        SchemaNodeContent::Union(_)
        | SchemaNodeContent::Any
        | SchemaNodeContent::Text(_)
        | SchemaNodeContent::Integer(_)
        | SchemaNodeContent::Float(_)
        | SchemaNodeContent::Boolean
        | SchemaNodeContent::Null
        | SchemaNodeContent::Literal(_)
        | SchemaNodeContent::Array(_)
        | SchemaNodeContent::Map(_)
        | SchemaNodeContent::Tuple(_)
        | SchemaNodeContent::Reference(_) => {}
    }
}

struct RecordVisitor<'s> {
    schema: &'s SchemaDocument,
    record_schema: &'s eure_schema::RecordSchema,
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for RecordVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("record")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        // Build a combined lookup that includes fields from flattened schemas.
        let mut flatten_fields: std::collections::HashMap<String, (SchemaNodeId, bool)> =
            std::collections::HashMap::new();
        for &fid in &self.record_schema.flatten {
            collect_flatten_fields(self.schema, fid, &mut flatten_fields);
        }

        self.constructor
            .bind_empty_map()
            .map_err(A::Error::custom)?;
        let mut seen = HashSet::new();

        while let Some(key) = map.next_key::<String>()? {
            // Check direct properties first, then flattened schemas.
            let schema_node_id = if let Some(field_schema) = self.record_schema.properties.get(&key)
            {
                Some(field_schema.schema)
            } else {
                flatten_fields.get(&key).map(|&(id, _opt)| id)
            };

            if let Some(schema_node_id) = schema_node_id {
                let scope = self.constructor.begin_scope();
                self.constructor
                    .navigate(PathSegment::Value(ObjectKey::String(key.clone())))
                    .map_err(A::Error::custom)?;
                map.next_value_seed(EureDocumentSeed {
                    schema: self.schema,
                    schema_node_id,
                    constructor: self.constructor,
                })?;
                self.constructor
                    .end_scope(scope)
                    .map_err(A::Error::custom)?;
                seen.insert(key);
                continue;
            }

            match &self.record_schema.unknown_fields {
                UnknownFieldsPolicy::Deny => {
                    return Err(A::Error::custom(format!("unknown field: {key}")));
                }
                UnknownFieldsPolicy::Allow => {
                    let scope = self.constructor.begin_scope();
                    self.constructor
                        .navigate(PathSegment::Value(ObjectKey::String(key.clone())))
                        .map_err(A::Error::custom)?;
                    map.next_value_seed(AnySeed {
                        constructor: self.constructor,
                    })?;
                    self.constructor
                        .end_scope(scope)
                        .map_err(A::Error::custom)?;
                }
                UnknownFieldsPolicy::Schema(schema_id) => {
                    let scope = self.constructor.begin_scope();
                    self.constructor
                        .navigate(PathSegment::Value(ObjectKey::String(key.clone())))
                        .map_err(A::Error::custom)?;
                    map.next_value_seed(EureDocumentSeed {
                        schema: self.schema,
                        schema_node_id: *schema_id,
                        constructor: self.constructor,
                    })?;
                    self.constructor
                        .end_scope(scope)
                        .map_err(A::Error::custom)?;
                }
            }
        }

        for (name, field) in &self.record_schema.properties {
            if !field.optional && !seen.contains(name) {
                return Err(A::Error::custom(
                    DeError::MissingField(name.clone()).to_string(),
                ));
            }
        }
        for (name, &(_id, optional)) in &flatten_fields {
            if !optional && !seen.contains(name) {
                return Err(A::Error::custom(
                    DeError::MissingField(name.clone()).to_string(),
                ));
            }
        }

        Ok(())
    }
}

struct ExternalVariantVisitor<'s> {
    schema: &'s SchemaDocument,
    union_schema: &'s UnionSchema,
    constructor: &'s mut DocumentConstructor,
}

impl<'de> Visitor<'de> for ExternalVariantVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an externally tagged union")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let variant_name: String = map
            .next_key()?
            .ok_or_else(|| A::Error::custom("expected variant map with one key"))?;

        let Some(&variant_schema_id) = self.union_schema.variants.get(&variant_name) else {
            return Err(A::Error::custom(format!("unknown variant: {variant_name}")));
        };

        self.constructor
            .set_variant(&variant_name)
            .map_err(A::Error::custom)?;
        map.next_value_seed(EureDocumentSeed {
            schema: self.schema,
            schema_node_id: variant_schema_id,
            constructor: self.constructor,
        })?;

        if map.next_key::<de::IgnoredAny>()?.is_some() {
            return Err(A::Error::custom(
                "expected externally tagged union map with exactly one key",
            ));
        }

        Ok(())
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if self.union_schema.variants.contains_key(value) {
            self.constructor.set_variant(value).map_err(E::custom)?;
            self.constructor
                .bind_primitive(PrimitiveValue::Null)
                .map_err(E::custom)?;
            Ok(())
        } else {
            Err(E::custom(format!("unknown variant: {value}")))
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&value)
    }
}

struct AnyObjectKeyVisitor;

impl<'de> Visitor<'de> for AnyObjectKeyVisitor {
    type Value = ObjectKey;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a Eure-compatible map key")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::from(value))
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(BigInt::from(value)))
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(BigInt::from(value)))
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(BigInt::from(value)))
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(BigInt::from(value)))
    }

    fn visit_char<E>(self, value: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_string()))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_owned()))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut elements = Vec::new();
        while let Some(value) = seq.next_element_seed(AnyObjectKeySeed)? {
            elements.push(value);
        }
        Ok(ObjectKey::Tuple(Tuple(elements)))
    }
}

struct TextObjectKeyVisitor;

impl<'de> Visitor<'de> for TextObjectKeyVisitor {
    type Value = ObjectKey;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a text map key")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(if value {
            "true".to_string()
        } else {
            "false".to_string()
        }))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_string()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_string()))
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_string()))
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_string()))
    }

    fn visit_char<E>(self, value: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_string()))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_owned()))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::String(value))
    }
}

struct IntegerObjectKeyVisitor;

impl<'de> Visitor<'de> for IntegerObjectKeyVisitor {
    type Value = ObjectKey;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an integer map key")
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(i64::from(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(BigInt::from(value)))
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(BigInt::from(value)))
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(u64::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(BigInt::from(value)))
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(BigInt::from(value)))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(parse_bigint(value).map_err(E::custom)?))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(parse_bigint(value).map_err(E::custom)?))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::Number(parse_bigint(&value).map_err(E::custom)?))
    }
}

struct BoolObjectKeyVisitor;

impl<'de> Visitor<'de> for BoolObjectKeyVisitor {
    type Value = ObjectKey;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a boolean map key")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ObjectKey::from(value))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "true" => Ok(ObjectKey::String(value.to_owned())),
            "false" => Ok(ObjectKey::String(value.to_owned())),
            _ => Err(E::custom(type_mismatch("boolean", "text").to_string())),
        }
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_borrowed_str(value)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&value)
    }
}

struct LiteralObjectKeyVisitor<'a> {
    expected: &'a EureDocument,
}

impl<'de> Visitor<'de> for LiteralObjectKeyVisitor<'_> {
    type Value = ObjectKey;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a literal map key")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match expected_primitive(self.expected).map_err(E::custom)? {
            PrimitiveValue::Bool(expected) if *expected == value => Ok(ObjectKey::from(value)),
            PrimitiveValue::Bool(_) => Err(E::custom("literal mismatch")),
            primitive => Err(E::custom(type_mismatch(
                "boolean",
                primitive_type_name(primitive),
            ))),
        }
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        literal_integer_key(self.expected, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        literal_integer_key(self.expected, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        literal_integer_key(self.expected, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        literal_integer_key(self.expected, BigInt::from(value)).map_err(E::custom)
    }

    fn visit_char<E>(self, value: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_string())
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        literal_string_key(self.expected, value).map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        literal_string_key(self.expected, value).map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        literal_string_key(self.expected, &value).map_err(E::custom)
    }
}

struct TupleObjectKeyVisitor<'s> {
    schema: &'s SchemaDocument,
    elements: &'s [SchemaNodeId],
}

impl<'de> Visitor<'de> for TupleObjectKeyVisitor<'_> {
    type Value = ObjectKey;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a tuple map key")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut keys = Vec::with_capacity(self.elements.len());

        for &element_schema_id in self.elements {
            let Some(key) = seq.next_element_seed(ObjectKeySeed {
                schema: self.schema,
                schema_node_id: element_schema_id,
            })?
            else {
                return Err(A::Error::custom(DeError::EndOfSequence.to_string()));
            };
            keys.push(key);
        }

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(A::Error::custom(format!(
                "tuple length mismatch: expected {}, got more elements",
                self.elements.len()
            )));
        }

        Ok(ObjectKey::Tuple(Tuple(keys)))
    }
}

struct UnionObjectKeyVisitor<'s> {
    schema: &'s SchemaDocument,
    union_schema: &'s UnionSchema,
}

impl<'de> Visitor<'de> for UnionObjectKeyVisitor<'_> {
    type Value = ObjectKey;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a union-compatible map key")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        union_key_from_bool(self.schema, self.union_schema, value).map_err(E::custom)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        union_key_from_integer(self.schema, self.union_schema, BigInt::from(value))
            .map_err(E::custom)
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        union_key_from_integer(self.schema, self.union_schema, BigInt::from(value))
            .map_err(E::custom)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        union_key_from_integer(self.schema, self.union_schema, BigInt::from(value))
            .map_err(E::custom)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        union_key_from_integer(self.schema, self.union_schema, BigInt::from(value))
            .map_err(E::custom)
    }

    fn visit_char<E>(self, value: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_string())
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        union_key_from_string(self.schema, self.union_schema, value).map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        union_key_from_string(self.schema, self.union_schema, value).map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        union_key_from_string(self.schema, self.union_schema, &value).map_err(E::custom)
    }
}

fn deserialize_union<'de, D: serde::Deserializer<'de>>(
    de: D,
    schema: &SchemaDocument,
    union_schema: &UnionSchema,
    _schema_node_id: SchemaNodeId,
    constructor: &mut DocumentConstructor,
) -> Result<(), D::Error> {
    let repr = union_schema
        .interop
        .variant_repr
        .as_ref()
        .unwrap_or(&VariantRepr::External);

    match repr {
        VariantRepr::External => de.deserialize_map(ExternalVariantVisitor {
            schema,
            union_schema,
            constructor,
        }),
        VariantRepr::Internal { tag } => {
            let content = de
                .deserialize_map(EureContentMapVisitor)
                .map_err(D::Error::custom)?;
            deserialize_internal_tagged(content, schema, union_schema, tag, constructor)
                .map_err(D::Error::custom)
        }
        VariantRepr::Adjacent {
            tag,
            content: content_key,
        } => {
            let content = de
                .deserialize_map(EureContentMapVisitor)
                .map_err(D::Error::custom)?;
            deserialize_adjacent_tagged(
                content,
                schema,
                union_schema,
                tag,
                content_key,
                constructor,
            )
            .map_err(D::Error::custom)
        }
        VariantRepr::Untagged => {
            // Untagged unions require a self-describing input so we can trial-match variants.
            let content = EureContent::deserialize(de).map_err(D::Error::custom)?;
            deserialize_untagged(content, schema, union_schema, constructor)
                .map_err(D::Error::custom)
        }
    }
}

fn deserialize_internal_tagged<'de>(
    content: EureContent<'de>,
    schema: &SchemaDocument,
    union_schema: &UnionSchema,
    tag: &str,
    constructor: &mut DocumentConstructor,
) -> Result<(), DeError> {
    let entries = match content {
        EureContent::Map(entries) => entries,
        other => return Err(type_mismatch("map", eure_content_type(&other))),
    };

    let mut variant_name = None;
    let mut remaining = Vec::new();

    for (key, value) in entries {
        if content_key_matches(&key, tag) {
            if variant_name.is_some() {
                return Err(DeError::Custom(format!("duplicate field: {tag}")));
            }
            variant_name = Some(content_as_variant_name(value)?);
        } else {
            remaining.push((key, value));
        }
    }

    let variant_name = variant_name.ok_or_else(|| DeError::MissingField(tag.to_string()))?;
    let variant_schema_id = *union_schema
        .variants
        .get(&variant_name)
        .ok_or_else(|| DeError::InvalidVariantName(variant_name.clone()))?;

    constructor.set_variant(&variant_name)?;

    let resolved_variant_id = resolve_schema_node_id(schema, variant_schema_id)?;
    let variant_content = if remaining.is_empty()
        && matches!(
            schema.node(resolved_variant_id).content,
            SchemaNodeContent::Null
        ) {
        EureContent::Unit
    } else {
        EureContent::Map(remaining)
    };

    EureDocumentSeed {
        schema,
        schema_node_id: variant_schema_id,
        constructor,
    }
    .deserialize(variant_content)
}

fn deserialize_adjacent_tagged<'de>(
    content: EureContent<'de>,
    schema: &SchemaDocument,
    union_schema: &UnionSchema,
    tag: &str,
    content_key: &str,
    constructor: &mut DocumentConstructor,
) -> Result<(), DeError> {
    let entries = match content {
        EureContent::Map(entries) => entries,
        other => return Err(type_mismatch("map", eure_content_type(&other))),
    };

    let mut variant_name = None;
    let mut variant_content = None;
    let mut remaining = Vec::new();

    for (key, value) in entries {
        if content_key_matches(&key, tag) {
            if variant_name.is_some() {
                return Err(DeError::Custom(format!("duplicate field: {tag}")));
            }
            variant_name = Some(content_as_variant_name(value)?);
        } else if content_key_matches(&key, content_key) {
            if variant_content.is_some() {
                return Err(DeError::Custom(format!("duplicate field: {content_key}")));
            }
            variant_content = Some(value);
        } else {
            remaining.push((key, value));
        }
    }

    let variant_name = variant_name.ok_or_else(|| DeError::MissingField(tag.to_string()))?;
    let variant_schema_id = *union_schema
        .variants
        .get(&variant_name)
        .ok_or_else(|| DeError::InvalidVariantName(variant_name.clone()))?;
    let resolved_variant_id = resolve_schema_node_id(schema, variant_schema_id)?;
    // Unit variants (Null schema) have no content field — treat missing content as Unit.
    let is_null_variant = matches!(
        schema.node(resolved_variant_id).content,
        SchemaNodeContent::Null
    );
    let variant_content = if is_null_variant {
        variant_content.unwrap_or(EureContent::Unit)
    } else {
        variant_content.ok_or_else(|| DeError::MissingField(content_key.to_string()))?
    };
    let variant_content = merge_adjacent_variant_content(
        variant_content,
        remaining,
        &schema.node(resolved_variant_id).content,
    )?;

    constructor.set_variant(&variant_name)?;
    EureDocumentSeed {
        schema,
        schema_node_id: variant_schema_id,
        constructor,
    }
    .deserialize(variant_content)
}

// Limitation: variant selection is based on structural deserialization success, not value-level
// constraints (ranges, patterns, literal equality). Untagged unions whose variants are only
// distinguishable by value (e.g., `int<=10 | int>=11`, `literal("ok") | text`) may match
// the wrong branch. Use explicit tagging or `unambiguous` semantics with structurally-distinct
// variants to avoid this. Running the schema validator afterward can detect but not repair
// a wrong variant assignment.
fn try_untagged_variant<'de>(
    content: EureContent<'de>,
    schema: &SchemaDocument,
    variant_schema_id: SchemaNodeId,
) -> Option<(EureDocument, Option<VariantPath>)> {
    let mut temp_constructor = DocumentConstructor::new();
    let attempt = EureDocumentSeed {
        schema,
        schema_node_id: variant_schema_id,
        constructor: &mut temp_constructor,
    }
    .deserialize(content.into_deserializer());

    if attempt.is_ok() {
        let mut temp_doc = temp_constructor.finish();
        let root_id = temp_doc.get_root_id();
        let inner_variant_path = extract_variant_path(&temp_doc, root_id).ok()?;
        temp_doc
            .node_mut(root_id)
            .extensions
            .remove_ordered(&Identifier::VARIANT);
        Some((temp_doc, inner_variant_path))
    } else {
        None
    }
}

fn deserialize_untagged<'de>(
    content: EureContent<'de>,
    schema: &SchemaDocument,
    union_schema: &UnionSchema,
    constructor: &mut DocumentConstructor,
) -> Result<(), DeError> {
    // First pass: priority (non-`unambiguous`) variants use first-match semantics —
    // matches the core parser's `variant()` registration with `is_priority=true`.
    for (variant_name, &variant_schema_id) in &union_schema.variants {
        if union_schema.deny_untagged.contains(variant_name)
            || union_schema.unambiguous.contains(variant_name)
        {
            continue;
        }

        if let Some((temp_doc, inner_variant_path)) =
            try_untagged_variant(content.clone(), schema, variant_schema_id)
        {
            let root_id = temp_doc.get_root_id();
            constructor.set_variant(variant_name)?;
            if let Some(inner_variant_path) = inner_variant_path
                && !inner_variant_path.is_empty()
            {
                constructor.set_variant(&inner_variant_path.to_string())?;
            }
            constructor.write_subtree(&temp_doc, root_id)?;
            return Ok(());
        }
    }

    // Second pass: `unambiguous` variants — try all, error on conflicts.
    // Matches the core parser's `variant_unambiguous()` with `is_priority=false`.
    let mut unambiguous_match: Option<(&str, EureDocument, Option<VariantPath>)> = None;
    for (variant_name, &variant_schema_id) in &union_schema.variants {
        if union_schema.deny_untagged.contains(variant_name)
            || !union_schema.unambiguous.contains(variant_name)
        {
            continue;
        }
        if let Some((doc, inner_path)) =
            try_untagged_variant(content.clone(), schema, variant_schema_id)
        {
            if unambiguous_match.is_some() {
                return Err(DeError::Custom(
                    "ambiguous untagged union: multiple variants match".to_string(),
                ));
            }
            unambiguous_match = Some((variant_name, doc, inner_path));
        }
    }
    if let Some((variant_name, temp_doc, inner_variant_path)) = unambiguous_match {
        let root_id = temp_doc.get_root_id();
        constructor.set_variant(variant_name)?;
        if let Some(inner_variant_path) = inner_variant_path
            && !inner_variant_path.is_empty()
        {
            constructor.set_variant(&inner_variant_path.to_string())?;
        }
        constructor.write_subtree(&temp_doc, root_id)?;
        return Ok(());
    }

    Err(DeError::NoVariantMatched)
}

fn merge_adjacent_variant_content<'de>(
    content: EureContent<'de>,
    mut sibling_entries: Vec<(EureContent<'de>, EureContent<'de>)>,
    schema_content: &SchemaNodeContent,
) -> Result<EureContent<'de>, DeError> {
    if sibling_entries.is_empty() {
        return Ok(content);
    }

    match schema_content {
        SchemaNodeContent::Record(_) | SchemaNodeContent::Map(_) | SchemaNodeContent::Union(_) => {
            match content {
                EureContent::Map(mut entries) => {
                    entries.append(&mut sibling_entries);
                    Ok(EureContent::Map(entries))
                }
                other => Err(DeError::TypeMismatch {
                    expected: "map",
                    actual: eure_content_type(&other).to_string(),
                }),
            }
        }
        _ => Err(DeError::Custom(
            "adjacent-tagged extra sibling fields require map-like variant content".to_string(),
        )),
    }
}

fn bind_text_value(constructor: &mut DocumentConstructor, value: String) -> Result<(), DeError> {
    constructor
        .bind_primitive(PrimitiveValue::Text(Text::plaintext(value)))
        .map_err(Into::into)
}

fn bind_integer_value(constructor: &mut DocumentConstructor, value: BigInt) -> Result<(), DeError> {
    constructor
        .bind_primitive(PrimitiveValue::Integer(value))
        .map_err(Into::into)
}

fn bind_f32_value(constructor: &mut DocumentConstructor, value: f32) -> Result<(), DeError> {
    constructor
        .bind_primitive(PrimitiveValue::F32(value))
        .map_err(Into::into)
}

fn bind_f64_value(constructor: &mut DocumentConstructor, value: f64) -> Result<(), DeError> {
    constructor
        .bind_primitive(PrimitiveValue::F64(value))
        .map_err(Into::into)
}

fn bind_bool_value(constructor: &mut DocumentConstructor, value: bool) -> Result<(), DeError> {
    constructor
        .bind_primitive(PrimitiveValue::Bool(value))
        .map_err(Into::into)
}

fn bind_null_value(constructor: &mut DocumentConstructor) -> Result<(), DeError> {
    constructor
        .bind_primitive(PrimitiveValue::Null)
        .map_err(Into::into)
}

fn bind_bytes_as_array(constructor: &mut DocumentConstructor, bytes: &[u8]) -> Result<(), DeError> {
    constructor.bind_empty_array()?;
    for &byte in bytes {
        let scope = constructor.begin_scope();
        constructor.navigate(PathSegment::ArrayIndex(None))?;
        constructor.bind_primitive(PrimitiveValue::Integer(BigInt::from(byte)))?;
        constructor.end_scope(scope)?;
    }
    Ok(())
}

fn parse_bigint(value: &str) -> Result<BigInt, DeError> {
    BigInt::parse_bytes(value.as_bytes(), 10).ok_or_else(|| type_mismatch("integer", "text"))
}

fn resolve_schema_node_id(
    schema: &SchemaDocument,
    mut schema_node_id: SchemaNodeId,
) -> Result<SchemaNodeId, DeError> {
    for _ in 0..schema.nodes.len() {
        match &schema.node(schema_node_id).content {
            SchemaNodeContent::Reference(type_ref) => {
                if let Some(namespace) = &type_ref.namespace {
                    return Err(DeError::Custom(format!(
                        "cross-schema references are unsupported: {namespace}.{}",
                        type_ref.name
                    )));
                }
                schema_node_id = schema.types.get(&type_ref.name).copied().ok_or_else(|| {
                    DeError::Custom(format!("undefined type reference: {}", type_ref.name))
                })?;
            }
            _ => return Ok(schema_node_id),
        }
    }

    Err(DeError::Custom(
        "schema reference cycle detected".to_string(),
    ))
}

fn extract_variant_path(
    doc: &EureDocument,
    node_id: eure::document::NodeId,
) -> Result<Option<VariantPath>, DeError> {
    let Some(variant_id) = doc
        .node(node_id)
        .extensions
        .get(&Identifier::VARIANT)
        .copied()
    else {
        return Ok(None);
    };
    let value = match &doc.node(variant_id).content {
        NodeValue::Primitive(PrimitiveValue::Text(text)) => text.as_str(),
        other => {
            return Err(DeError::TypeMismatch {
                expected: "text",
                actual: primitive_or_node_type_name(other).to_string(),
            });
        }
    };

    VariantPath::parse(value)
        .map(Some)
        .map_err(|_| DeError::InvalidVariantName(value.to_string()))
}

fn primitive_or_node_type_name(value: &NodeValue) -> &'static str {
    match value {
        NodeValue::Hole(_) => "hole",
        NodeValue::Primitive(PrimitiveValue::Null) => "null",
        NodeValue::Primitive(PrimitiveValue::Bool(_)) => "boolean",
        NodeValue::Primitive(PrimitiveValue::Integer(_)) => "integer",
        NodeValue::Primitive(PrimitiveValue::F32(_))
        | NodeValue::Primitive(PrimitiveValue::F64(_)) => "float",
        NodeValue::Primitive(PrimitiveValue::Text(_)) => "text",
        NodeValue::Array(_) => "array",
        NodeValue::Map(_) => "map",
        NodeValue::Tuple(_) => "tuple",
        NodeValue::PartialMap(_) => "partial-map",
    }
}

fn content_key_matches(content: &EureContent<'_>, expected: &str) -> bool {
    match content {
        EureContent::String(value) => value.as_ref() == expected,
        EureContent::Char(value) => {
            let mut chars = expected.chars();
            chars.next() == Some(*value) && chars.next().is_none()
        }
        _ => false,
    }
}

fn content_as_variant_name(content: EureContent<'_>) -> Result<String, DeError> {
    match content {
        EureContent::String(value) => Ok(value.into_owned()),
        EureContent::Char(value) => Ok(value.to_string()),
        other => Err(type_mismatch("text", eure_content_type(&other))),
    }
}

fn type_mismatch(expected: &'static str, actual: impl Into<String>) -> DeError {
    DeError::TypeMismatch {
        expected,
        actual: actual.into(),
    }
}

fn primitive_type_name(primitive: &PrimitiveValue) -> &'static str {
    match primitive {
        PrimitiveValue::Null => "null",
        PrimitiveValue::Bool(_) => "boolean",
        PrimitiveValue::Integer(_) => "integer",
        PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => "float",
        PrimitiveValue::Text(_) => "text",
    }
}

fn eure_content_type(content: &EureContent<'_>) -> String {
    match content {
        EureContent::Bool(_) => "boolean".to_string(),
        EureContent::I8(_)
        | EureContent::I16(_)
        | EureContent::I32(_)
        | EureContent::I64(_)
        | EureContent::I128(_)
        | EureContent::U8(_)
        | EureContent::U16(_)
        | EureContent::U32(_)
        | EureContent::U64(_)
        | EureContent::U128(_) => "integer".to_string(),
        EureContent::F32(_) | EureContent::F64(_) => "float".to_string(),
        EureContent::Char(_) | EureContent::String(_) => "text".to_string(),
        EureContent::Bytes(_) => "bytes".to_string(),
        EureContent::Unit => "null".to_string(),
        EureContent::Seq(_) => "array".to_string(),
        EureContent::Map(_) => "map".to_string(),
    }
}

fn expected_primitive(expected: &EureDocument) -> Result<&PrimitiveValue, DeError> {
    expected
        .root()
        .as_primitive()
        .ok_or_else(unsupported_complex_literal_key)
}

fn unsupported_complex_map_keys() -> DeError {
    DeError::Custom("complex map keys are unsupported in serde-eure v1".to_string())
}

fn unsupported_complex_literal_key() -> DeError {
    DeError::Custom("complex literal map keys are unsupported in serde-eure v1".to_string())
}

fn literal_integer_key(expected: &EureDocument, actual: BigInt) -> Result<ObjectKey, DeError> {
    match expected_primitive(expected)? {
        PrimitiveValue::Integer(expected_value) if expected_value == &actual => {
            Ok(ObjectKey::Number(actual))
        }
        PrimitiveValue::Integer(_) => Err(DeError::Custom("literal mismatch".to_string())),
        primitive => Err(type_mismatch("integer", primitive_type_name(primitive))),
    }
}

fn literal_string_key(expected: &EureDocument, actual: &str) -> Result<ObjectKey, DeError> {
    match expected_primitive(expected)? {
        PrimitiveValue::Text(expected_value) if expected_value.as_str() == actual => {
            Ok(ObjectKey::String(actual.to_owned()))
        }
        PrimitiveValue::Text(_) => Err(DeError::Custom("literal mismatch".to_string())),
        PrimitiveValue::Integer(expected_value) => {
            let parsed = parse_bigint(actual)?;
            if expected_value == &parsed {
                Ok(ObjectKey::Number(parsed))
            } else {
                Err(DeError::Custom("literal mismatch".to_string()))
            }
        }
        PrimitiveValue::Bool(expected_value) => {
            let expected_text = if *expected_value { "true" } else { "false" };
            if actual == expected_text {
                Ok(ObjectKey::String(actual.to_owned()))
            } else {
                Err(DeError::Custom("literal mismatch".to_string()))
            }
        }
        PrimitiveValue::Null | PrimitiveValue::F32(_) | PrimitiveValue::F64(_) => {
            Err(unsupported_complex_literal_key())
        }
    }
}

fn object_key_from_string_for_schema(
    schema: &SchemaDocument,
    schema_node_id: SchemaNodeId,
    value: &str,
) -> Result<ObjectKey, DeError> {
    let schema_node_id = resolve_schema_node_id(schema, schema_node_id)?;

    match &schema.node(schema_node_id).content {
        SchemaNodeContent::Any | SchemaNodeContent::Text(_) => {
            Ok(ObjectKey::String(value.to_owned()))
        }
        SchemaNodeContent::Integer(_) => Ok(ObjectKey::Number(parse_bigint(value)?)),
        SchemaNodeContent::Boolean => match value {
            "true" | "false" => Ok(ObjectKey::String(value.to_owned())),
            _ => Err(type_mismatch("boolean", "text")),
        },
        SchemaNodeContent::Literal(expected) => literal_string_key(expected, value),
        SchemaNodeContent::Tuple(_)
        | SchemaNodeContent::Float(_)
        | SchemaNodeContent::Null
        | SchemaNodeContent::Array(_)
        | SchemaNodeContent::Map(_)
        | SchemaNodeContent::Record(_) => Err(unsupported_complex_map_keys()),
        SchemaNodeContent::Union(union_schema) => {
            union_key_from_string(schema, union_schema, value)
        }
        SchemaNodeContent::Reference(_) => unreachable!("references are resolved above"),
    }
}

fn object_key_from_integer_for_schema(
    schema: &SchemaDocument,
    schema_node_id: SchemaNodeId,
    value: &BigInt,
) -> Result<ObjectKey, DeError> {
    let schema_node_id = resolve_schema_node_id(schema, schema_node_id)?;

    match &schema.node(schema_node_id).content {
        SchemaNodeContent::Any | SchemaNodeContent::Integer(_) => {
            Ok(ObjectKey::Number(value.clone()))
        }
        SchemaNodeContent::Text(_) => Ok(ObjectKey::String(value.to_string())),
        SchemaNodeContent::Literal(expected) => literal_integer_key(expected, value.clone()),
        SchemaNodeContent::Union(union_schema) => {
            union_key_from_integer(schema, union_schema, value.clone())
        }
        SchemaNodeContent::Boolean
        | SchemaNodeContent::Tuple(_)
        | SchemaNodeContent::Float(_)
        | SchemaNodeContent::Null
        | SchemaNodeContent::Array(_)
        | SchemaNodeContent::Map(_)
        | SchemaNodeContent::Record(_) => Err(unsupported_complex_map_keys()),
        SchemaNodeContent::Reference(_) => unreachable!("references are resolved above"),
    }
}

fn object_key_from_bool_for_schema(
    schema: &SchemaDocument,
    schema_node_id: SchemaNodeId,
    value: bool,
) -> Result<ObjectKey, DeError> {
    let schema_node_id = resolve_schema_node_id(schema, schema_node_id)?;

    match &schema.node(schema_node_id).content {
        SchemaNodeContent::Any | SchemaNodeContent::Text(_) | SchemaNodeContent::Boolean => {
            Ok(ObjectKey::from(value))
        }
        SchemaNodeContent::Literal(expected) => match expected_primitive(expected)? {
            PrimitiveValue::Bool(expected_value) if *expected_value == value => {
                Ok(ObjectKey::from(value))
            }
            PrimitiveValue::Bool(_) => Err(DeError::Custom("literal mismatch".to_string())),
            primitive => Err(type_mismatch("boolean", primitive_type_name(primitive))),
        },
        SchemaNodeContent::Union(union_schema) => union_key_from_bool(schema, union_schema, value),
        SchemaNodeContent::Integer(_)
        | SchemaNodeContent::Tuple(_)
        | SchemaNodeContent::Float(_)
        | SchemaNodeContent::Null
        | SchemaNodeContent::Array(_)
        | SchemaNodeContent::Map(_)
        | SchemaNodeContent::Record(_) => Err(unsupported_complex_map_keys()),
        SchemaNodeContent::Reference(_) => unreachable!("references are resolved above"),
    }
}

fn union_key_from_string(
    schema: &SchemaDocument,
    union_schema: &UnionSchema,
    value: &str,
) -> Result<ObjectKey, DeError> {
    if union_schema.variants.contains_key(value) {
        return Ok(ObjectKey::String(value.to_owned()));
    }

    for &variant_schema_id in union_schema.variants.values() {
        if let Ok(key) = object_key_from_string_for_schema(schema, variant_schema_id, value) {
            return Ok(key);
        }
    }

    Err(type_mismatch("map-key", "text"))
}

fn union_key_from_integer(
    schema: &SchemaDocument,
    union_schema: &UnionSchema,
    value: BigInt,
) -> Result<ObjectKey, DeError> {
    for &variant_schema_id in union_schema.variants.values() {
        if let Ok(key) = object_key_from_integer_for_schema(schema, variant_schema_id, &value) {
            return Ok(key);
        }
    }

    Err(type_mismatch("map-key", "integer"))
}

fn union_key_from_bool(
    schema: &SchemaDocument,
    union_schema: &UnionSchema,
    value: bool,
) -> Result<ObjectKey, DeError> {
    for &variant_schema_id in union_schema.variants.values() {
        if let Ok(key) = object_key_from_bool_for_schema(schema, variant_schema_id, value) {
            return Ok(key);
        }
    }

    Err(type_mismatch("map-key", "boolean"))
}

#[cfg(test)]
mod tests {
    use eure::eure;
    use eure_schema::TextSchema;
    use eure_schema::interop::{UnionInterop, VariantRepr};
    use eure_schema::{
        IntegerSchema, RecordFieldSchema, RecordSchema, SchemaDocument, SchemaNodeContent,
        UnionSchema, UnknownFieldsPolicy,
    };
    use serde_json::Deserializer;

    use super::from_deserializer;

    fn make_record_schema() -> SchemaDocument {
        let mut schema = SchemaDocument::new();
        let text_id = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let int_id = schema.create_node(SchemaNodeContent::Integer(IntegerSchema::default()));
        let record_id = schema.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [
                (
                    "name".to_string(),
                    RecordFieldSchema {
                        schema: text_id,
                        optional: false,
                        binding_style: None,
                        field_codegen: Default::default(),
                    },
                ),
                (
                    "age".to_string(),
                    RecordFieldSchema {
                        schema: int_id,
                        optional: false,
                        binding_style: None,
                        field_codegen: Default::default(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            flatten: Vec::new(),
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        schema.root = record_id;
        schema
    }

    fn make_union_schema(variant_repr: VariantRepr) -> SchemaDocument {
        let mut schema = SchemaDocument::new();
        let text_id = schema.create_node(SchemaNodeContent::Text(TextSchema::default()));
        let success_record_id = schema.create_node(SchemaNodeContent::Record(RecordSchema {
            properties: [(
                "message".to_string(),
                RecordFieldSchema {
                    schema: text_id,
                    optional: false,
                    binding_style: None,
                    field_codegen: Default::default(),
                },
            )]
            .into_iter()
            .collect(),
            flatten: Vec::new(),
            unknown_fields: UnknownFieldsPolicy::Deny,
        }));
        let union_id = schema.create_node(SchemaNodeContent::Union(UnionSchema {
            variants: [("success".to_string(), success_record_id)]
                .into_iter()
                .collect(),
            unambiguous: Default::default(),
            interop: UnionInterop {
                variant_repr: Some(variant_repr),
            },
            deny_untagged: Default::default(),
        }));
        schema.root = union_id;
        schema
    }

    #[test]
    fn deserializes_json_object_into_record_document() {
        let schema = make_record_schema();
        let mut de = Deserializer::from_str(r#"{ "name": "Alice", "age": 30 }"#);

        let actual = from_deserializer(&mut de, &schema).unwrap();
        let expected = eure!({
            name = "Alice",
            age = 30,
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserializes_external_variant_into_document() {
        let schema = make_union_schema(VariantRepr::External);
        let mut de = Deserializer::from_str(r#"{ "success": { "message": "ok" } }"#);

        let actual = from_deserializer(&mut de, &schema).unwrap();
        let expected = eure!({
            message = "ok",
            %variant = "success",
        });

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserializes_internal_variant_into_document() {
        let schema = make_union_schema(VariantRepr::Internal {
            tag: "type".to_string(),
        });
        let mut de = Deserializer::from_str(r#"{ "type": "success", "message": "ok" }"#);

        let actual = from_deserializer(&mut de, &schema).unwrap();
        let expected = eure!({
            message = "ok",
            %variant = "success",
        });

        assert_eq!(actual, expected);
    }
}
