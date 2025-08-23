use crate::error::{Error, Result};
use eure_value::value::{Array, Code, KeyCmpValue, Map, Tuple, Value, Variant};
use serde::Deserialize;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

pub struct Deserializer {
    value: Value,
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    // Parse EURE string to CST
    let tree = eure_parol::parse(s).map_err(|e| Error::ParseError(e.to_string()))?;

    // Extract values using ValueVisitor
    let mut visitor = eure_tree::value_visitor::ValueVisitor::new(s);
    tree.visit_from_root(&mut visitor)
        .map_err(|e| Error::ValueVisitorError(e.to_string()))?;
    let document = visitor.into_document();

    // Convert document to value
    let value = document.to_value();

    // Deserialize from Value
    from_value(value)
}

pub fn from_value<'a, T>(value: Value) -> Result<T>
where
    T: Deserialize<'a>,
{
    // Handle the special case where EURE wraps bare values in a root binding
    let unwrapped_value = if let Value::Map(Map(ref map)) = value {
        if map.len() == 1 && map.contains_key(&KeyCmpValue::String("value".to_string())) {
            // If we have a single "value" binding at the root, unwrap it
            map.get(&KeyCmpValue::String("value".to_string())).cloned()
        } else {
            None
        }
    } else {
        None
    };
    
    let final_value = unwrapped_value.unwrap_or(value);
    let mut deserializer = Deserializer::new(final_value);
    T::deserialize(&mut deserializer)
}

impl Deserializer {
    fn new(value: Value) -> Self {
        Deserializer { value }
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::I64(i) => visitor.visit_i64(*i),
            Value::U64(u) => visitor.visit_u64(*u),
            Value::F32(f) => visitor.visit_f32(*f),
            Value::F64(f) => visitor.visit_f64(*f),
            Value::String(s) => visitor.visit_string(s.clone()),
            Value::Code(Code { content, .. }) => visitor.visit_string(content.clone()),
            Value::CodeBlock(Code { content, .. }) => visitor.visit_string(content.clone()),
            Value::Array(_) => self.deserialize_seq(visitor),
            Value::Tuple(_) => self.deserialize_tuple(0, visitor),
            Value::Map(_) => self.deserialize_map(visitor),
            Value::Variant(_) => self.deserialize_enum("", &[], visitor),
            Value::Unit => visitor.visit_unit(),
            Value::Hole => {
                // Holes should be caught and reported during validation
                // For now, return an error during deserialization
                Err(Error::Message(
                    "Cannot deserialize hole value (!) - holes must be filled with actual values"
                        .to_string(),
                ))
            }
            Value::MetaExtension(meta) => {
                // MetaExtensions serialize as strings with $$ prefix
                visitor.visit_string(format!("$${}", meta))
            }
            Value::Path(path) => {
                // Convert path to string representation, skipping extensions
                let mut path_parts = Vec::new();
                let mut i = 0;

                while i < path.0.len() {
                    match &path.0[i] {
                        eure_value::value::PathSegment::Ident(id) => {
                            // Check if next segment is ArrayIndex
                            if i + 1 < path.0.len()
                                && let eure_value::value::PathSegment::ArrayIndex(idx) =
                                    &path.0[i + 1]
                            {
                                // Combine identifier with array index
                                if let Some(index) = idx {
                                    path_parts.push(format!("{}[{}]", id.as_ref(), index));
                                } else {
                                    path_parts.push(format!("{}[]", id.as_ref()));
                                }
                                i += 2; // Skip the ArrayIndex segment
                                continue;
                            }
                            path_parts.push(id.as_ref().to_string());
                        }
                        eure_value::value::PathSegment::Extension(_) => {
                            // Extensions are metadata, not data - skip in serialization
                            i += 1;
                            continue;
                        }
                        eure_value::value::PathSegment::MetaExt(_) => {
                            // Meta-extensions are metadata, not data - skip in serialization
                            i += 1;
                            continue;
                        }
                        eure_value::value::PathSegment::Value(v) => {
                            path_parts.push(format!("{v:?}"))
                        }
                        eure_value::value::PathSegment::TupleIndex(idx) => {
                            path_parts.push(idx.to_string())
                        }
                        eure_value::value::PathSegment::ArrayIndex(idx) => {
                            // Standalone array index (shouldn't normally happen after an ident)
                            if let Some(index) = idx {
                                path_parts.push(format!("[{index}]"));
                            } else {
                                path_parts.push("[]".to_string());
                            }
                        }
                    }
                    i += 1;
                }

                let path_str = path_parts.join(".");
                visitor.visit_string(format!(".{path_str}"))
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Bool(b) => visitor.visit_bool(*b),
            _ => Err(Error::InvalidType(format!(
                "expected bool, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::I64(i) => visitor.visit_i8(*i as i8),
            _ => Err(Error::InvalidType(format!(
                "expected i8, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::I64(i) => visitor.visit_i16(*i as i16),
            _ => Err(Error::InvalidType(format!(
                "expected i16, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::I64(i) => visitor.visit_i32(*i as i32),
            _ => Err(Error::InvalidType(format!(
                "expected i32, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::I64(i) => visitor.visit_i64(*i),
            _ => Err(Error::InvalidType(format!(
                "expected i64, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::U64(u) => visitor.visit_u8(*u as u8),
            Value::I64(i) if *i >= 0 && *i <= u8::MAX as i64 => visitor.visit_u8(*i as u8),
            _ => Err(Error::InvalidType(format!(
                "expected u8, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::U64(u) => visitor.visit_u16(*u as u16),
            Value::I64(i) if *i >= 0 && *i <= u16::MAX as i64 => visitor.visit_u16(*i as u16),
            _ => Err(Error::InvalidType(format!(
                "expected u16, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::U64(u) => visitor.visit_u32(*u as u32),
            Value::I64(i) if *i >= 0 && *i <= u32::MAX as i64 => visitor.visit_u32(*i as u32),
            _ => Err(Error::InvalidType(format!(
                "expected u32, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::U64(u) => visitor.visit_u64(*u),
            Value::I64(i) if *i >= 0 => visitor.visit_u64(*i as u64),
            _ => Err(Error::InvalidType(format!(
                "expected u64, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::F32(f) => visitor.visit_f32(*f),
            Value::F64(f) => visitor.visit_f32(*f as f32),
            Value::I64(i) => visitor.visit_f32(*i as f32),
            Value::U64(u) => visitor.visit_f32(*u as f32),
            _ => Err(Error::InvalidType(format!(
                "expected f32, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::F64(f) => visitor.visit_f64(*f),
            Value::F32(f) => visitor.visit_f64(*f as f64),
            Value::I64(i) => visitor.visit_f64(*i as f64),
            Value::U64(u) => visitor.visit_f64(*u as f64),
            _ => Err(Error::InvalidType(format!(
                "expected f64, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::String(s) if s.len() == 1 => visitor.visit_char(s.chars().next().unwrap()),
            _ => Err(Error::InvalidType(format!(
                "expected char, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::String(s) => visitor.visit_str(s),
            Value::Code(Code { content, .. }) => visitor.visit_str(content),
            Value::CodeBlock(Code { content, .. }) => visitor.visit_str(content),
            // Special handling for wrapped values (e.g., "value = ...")
            Value::Map(map) if map.0.len() == 1 => {
                if let Some(Value::String(s)) = map.0.get(&KeyCmpValue::String("value".to_string())) {
                    visitor.visit_str(s)
                } else {
                    Err(Error::InvalidType(format!(
                        "expected string, found {:?}",
                        self.value
                    )))
                }
            }
            _ => Err(Error::InvalidType(format!(
                "expected string, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::String(s) => visitor.visit_string(s.clone()),
            Value::Code(Code { content, .. }) => visitor.visit_string(content.clone()),
            Value::CodeBlock(Code { content, .. }) => visitor.visit_string(content.clone()),
            // Special handling for wrapped values (e.g., "value = ...")
            Value::Map(map) if map.0.len() == 1 => {
                if let Some(Value::String(s)) = map.0.get(&KeyCmpValue::String("value".to_string())) {
                    visitor.visit_string(s.clone())
                } else {
                    Err(Error::InvalidType(format!(
                        "expected string, found {:?}",
                        self.value
                    )))
                }
            }
            _ => Err(Error::InvalidType(format!(
                "expected string, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Array(Array(values)) => {
                let bytes: Result<Vec<u8>> = values
                    .iter()
                    .map(|v| match v {
                        Value::U64(u) if *u <= 255 => Ok(*u as u8),
                        _ => Err(Error::InvalidType("expected array of bytes".to_string())),
                    })
                    .collect();
                visitor.visit_bytes(&bytes?)
            }
            _ => Err(Error::InvalidType(format!(
                "expected bytes, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            Value::Unit | Value::Null => visitor.visit_unit(),
            Value::Tuple(Tuple(values)) if values.is_empty() => visitor.visit_unit(),
            _ => Err(Error::InvalidType(format!(
                "expected unit, found {:?}",
                self.value
            ))),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match std::mem::replace(&mut self.value, Value::Null) {
            Value::Array(Array(values)) => visitor.visit_seq(SeqDeserializer::new(values)),
            _ => Err(Error::InvalidType("expected array".to_string())),
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match std::mem::replace(&mut self.value, Value::Null) {
            Value::Tuple(Tuple(values)) => {
                if len != 0 && values.len() != len {
                    return Err(Error::InvalidType(format!(
                        "expected tuple of length {}, found {}",
                        len,
                        values.len()
                    )));
                }
                visitor.visit_seq(SeqDeserializer::new(values))
            }
            Value::Array(Array(values)) => {
                if values.len() != len {
                    return Err(Error::InvalidType(format!(
                        "expected tuple of length {}, found array of length {}",
                        len,
                        values.len()
                    )));
                }
                visitor.visit_seq(SeqDeserializer::new(values))
            }
            _ => Err(Error::InvalidType("expected tuple".to_string())),
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match std::mem::replace(&mut self.value, Value::Null) {
            Value::Map(Map(map)) => visitor.visit_map(MapDeserializer::new(map)),
            _ => Err(Error::InvalidType("expected map".to_string())),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // For internally tagged enums, serde handles the tag extraction
        // Just treat it as a regular map
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match std::mem::replace(&mut self.value, Value::Null) {
            Value::Variant(variant) => visitor.visit_enum(EnumDeserializer::new(variant)),
            Value::Map(Map(map))
                if map.contains_key(&KeyCmpValue::String("$variant".to_string())) =>
            {
                // Handle map-based enum representation (external tagging)
                // Put the value back for the enum access to use
                self.value = Value::Map(Map(map));
                visitor.visit_enum(self)
            }
            value => {
                // For untagged enums, pass the value directly
                self.value = value;
                visitor.visit_enum(self)
            }
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct SeqDeserializer {
    iter: std::vec::IntoIter<Value>,
}

impl SeqDeserializer {
    fn new(values: Vec<Value>) -> Self {
        SeqDeserializer {
            iter: values.into_iter(),
        }
    }
}

impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let mut deserializer = Deserializer::new(value);
                seed.deserialize(&mut deserializer).map(Some)
            }
            None => Ok(None),
        }
    }
}

struct MapDeserializer {
    iter: std::vec::IntoIter<(KeyCmpValue, Value)>,
    value: Option<Value>,
}

impl MapDeserializer {
    fn new(map: ahash::AHashMap<KeyCmpValue, Value>) -> Self {
        MapDeserializer {
            iter: map.into_iter().collect::<Vec<_>>().into_iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                let key_value = key_cmp_to_value(key);
                let mut deserializer = Deserializer::new(key_value);
                seed.deserialize(&mut deserializer).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                let mut deserializer = Deserializer::new(value);
                seed.deserialize(&mut deserializer)
            }
            None => Err(Error::Message("value called before key".to_string())),
        }
    }
}

struct EnumDeserializer {
    variant: Variant,
}

impl EnumDeserializer {
    fn new(variant: Variant) -> Self {
        EnumDeserializer { variant }
    }
}

impl<'de> de::EnumAccess<'de> for EnumDeserializer {
    type Error = Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let variant_name = Value::String(self.variant.tag.clone());
        let mut deserializer = Deserializer::new(variant_name);
        let variant_index = seed.deserialize(&mut deserializer)?;
        Ok((
            variant_index,
            VariantDeserializer {
                content: *self.variant.content,
            },
        ))
    }
}

// Also implement EnumAccess for Deserializer (for untagged enums)
impl<'de> de::EnumAccess<'de> for &mut Deserializer {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // For map-based enums with $variant, extract the tag for variant matching
        if let Value::Map(Map(map)) = &self.value
            && let Some(Value::String(tag)) = map.get(&KeyCmpValue::String("$variant".to_string()))
        {
            let tag_value = Value::String(tag.clone());
            let mut tag_deserializer = Deserializer::new(tag_value);
            let variant_index = seed.deserialize(&mut tag_deserializer)?;
            return Ok((variant_index, self));
        }

        let value = seed.deserialize(&mut *self)?;
        Ok((value, self))
    }
}

struct VariantDeserializer {
    content: Value,
}

impl<'de> de::VariantAccess<'de> for VariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        match self.content {
            Value::Unit | Value::Null => Ok(()),
            _ => Err(Error::InvalidType("expected unit variant".to_string())),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let mut deserializer = Deserializer::new(self.content);
        seed.deserialize(&mut deserializer)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut deserializer = Deserializer::new(self.content);
        de::Deserializer::deserialize_tuple(&mut deserializer, 0, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut deserializer = Deserializer::new(self.content);
        de::Deserializer::deserialize_map(&mut deserializer, visitor)
    }
}

// Also implement VariantAccess for Deserializer (for untagged enums)
impl<'de> de::VariantAccess<'de> for &mut Deserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        // For map-based enums, the map should only contain $variant for unit variants
        match &self.value {
            Value::Map(Map(map))
                if map.len() == 1
                    && map.contains_key(&KeyCmpValue::String("$variant".to_string())) =>
            {
                Ok(())
            }
            Value::Unit | Value::Null => Ok(()),
            _ => Err(Error::InvalidType(format!(
                "expected unit variant, found {:?}",
                self.value
            ))),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        // For map-based enums with $content, extract it
        if let Value::Map(Map(map)) = &self.value
            && let Some(content) = map.get(&KeyCmpValue::String("$content".to_string()))
        {
            let content_value = content.clone();
            self.value = content_value;
        }
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // For map-based enums with $values, extract it
        if let Value::Map(Map(map)) = &self.value
            && let Some(values) = map.get(&KeyCmpValue::String("$values".to_string()))
        {
            let values_value = values.clone();
            self.value = values_value;
        }
        de::Deserializer::deserialize_tuple(self, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // For map-based enums, we need to remove the $variant field and deserialize the rest
        if let Value::Map(Map(map)) = &mut self.value
            && map.contains_key(&KeyCmpValue::String("$variant".to_string()))
        {
            let mut content_map = map.clone();
            content_map.remove(&KeyCmpValue::String("$variant".to_string()));
            self.value = Value::Map(Map(content_map));
        }
        de::Deserializer::deserialize_struct(self, "", fields, visitor)
    }
}

fn key_cmp_to_value(key: KeyCmpValue) -> Value {
    match key {
        KeyCmpValue::Null => Value::Null,
        KeyCmpValue::Bool(b) => Value::Bool(b),
        KeyCmpValue::I64(i) => Value::I64(i),
        KeyCmpValue::U64(u) => Value::U64(u),
        KeyCmpValue::String(s) => Value::String(s),
        KeyCmpValue::Tuple(Tuple(keys)) => {
            let values = keys.into_iter().map(key_cmp_to_value).collect();
            Value::Tuple(eure_value::value::Tuple(values))
        }
        KeyCmpValue::Unit => Value::Unit,
        KeyCmpValue::MetaExtension(meta) => Value::MetaExtension(meta),
        KeyCmpValue::Hole => Value::Hole,
    }
}
