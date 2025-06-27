use crate::error::{Error, Result};
use eure_value::value::{Array, KeyCmpValue, Map, Tuple, Value};
use serde::ser::{self, Serialize};

pub struct Serializer {
    // Configuration options could go here
}

pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: Serialize,
{
    let mut serializer = Serializer::new();
    value.serialize(&mut serializer)
}

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let eure_value = to_value(value)?;
    Ok(crate::format::format_eure_bindings(&eure_value))
}

pub fn to_string_pretty<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    // For now, same as to_string. Could add pretty printing options later.
    to_string(value)
}

impl Serializer {
    fn new() -> Self {
        Serializer {}
    }
}

impl ser::Serializer for &mut Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeTuple;
    type SerializeTupleStruct = SerializeTupleStruct;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeStruct;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Value> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(Value::I64(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Value> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Value> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Value> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Value> {
        Ok(Value::U64(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Value> {
        Ok(Value::F32(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Value> {
        Ok(Value::F64(v))
    }

    fn serialize_char(self, v: char) -> Result<Value> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        // Serialize bytes as an array of u8 values
        let values = v.iter().map(|&b| Value::U64(b as u64)).collect();
        Ok(Value::Array(Array(values)))
    }

    fn serialize_none(self) -> Result<Value> {
        Ok(Value::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Unit)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        // For externally tagged enums, serialize with our own format
        // For internally tagged enums, serde will call serialize_struct instead
        let mut map = ahash::AHashMap::new();
        map.insert(KeyCmpValue::String("$variant".to_string()), Value::String(variant.to_string()));
        Ok(Value::Map(Map(map)))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        let content = value.serialize(&mut *self)?;
        let mut map = ahash::AHashMap::new();
        map.insert(KeyCmpValue::String("$variant".to_string()), Value::String(variant.to_string()));
        map.insert(KeyCmpValue::String("$content".to_string()), content);
        Ok(Value::Map(Map(map)))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeSeq {
            values: Vec::new(),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Ok(SerializeTuple {
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SerializeTupleStruct {
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(SerializeTupleVariant {
            tag: variant.to_string(),
            values: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeMap {
            map: ahash::AHashMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Ok(SerializeStruct {
            map: ahash::AHashMap::new(),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(SerializeStructVariant {
            tag: variant.to_string(),
            map: ahash::AHashMap::new(),
        })
    }
}

pub struct SerializeSeq {
    values: Vec<Value>,
}

impl ser::SerializeSeq for SerializeSeq {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        self.values.push(value.serialize(&mut serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Array(Array(self.values)))
    }
}

pub struct SerializeTuple {
    values: Vec<Value>,
}

impl ser::SerializeTuple for SerializeTuple {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        self.values.push(value.serialize(&mut serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Tuple(Tuple(self.values)))
    }
}

pub struct SerializeTupleStruct {
    values: Vec<Value>,
}

impl ser::SerializeTupleStruct for SerializeTupleStruct {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        self.values.push(value.serialize(&mut serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Tuple(Tuple(self.values)))
    }
}

pub struct SerializeTupleVariant {
    tag: String,
    values: Vec<Value>,
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        self.values.push(value.serialize(&mut serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        // Create a map with tag and values for inline compatibility
        let mut map = ahash::AHashMap::new();
        map.insert(KeyCmpValue::String("$variant".to_string()), Value::String(self.tag));
        map.insert(KeyCmpValue::String("$values".to_string()), Value::Tuple(Tuple(self.values)));
        Ok(Value::Map(Map(map)))
    }
}

pub struct SerializeMap {
    map: ahash::AHashMap<KeyCmpValue, Value>,
    next_key: Option<KeyCmpValue>,
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        let key_value = key.serialize(&mut serializer)?;
        self.next_key = Some(value_to_key_cmp(key_value)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        let value = value.serialize(&mut serializer)?;
        let key = self.next_key.take()
            .ok_or_else(|| Error::Message("serialize_value called before serialize_key".to_string()))?;
        self.map.insert(key, value);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(Map(self.map)))
    }
}

pub struct SerializeStruct {
    map: ahash::AHashMap<KeyCmpValue, Value>,
}

impl ser::SerializeStruct for SerializeStruct {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        let value = value.serialize(&mut serializer)?;
        self.map.insert(KeyCmpValue::String(key.to_string()), value);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(Map(self.map)))
    }
}

pub struct SerializeStructVariant {
    tag: String,
    map: ahash::AHashMap<KeyCmpValue, Value>,
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        let value = value.serialize(&mut serializer)?;
        self.map.insert(KeyCmpValue::String(key.to_string()), value);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        // Add the variant tag
        let mut map = self.map;
        map.insert(KeyCmpValue::String("$variant".to_string()), Value::String(self.tag));
        Ok(Value::Map(Map(map)))
    }
}

fn value_to_key_cmp(value: Value) -> Result<KeyCmpValue> {
    match value {
        Value::Null => Ok(KeyCmpValue::Null),
        Value::Bool(b) => Ok(KeyCmpValue::Bool(b)),
        Value::I64(i) => Ok(KeyCmpValue::I64(i)),
        Value::U64(u) => Ok(KeyCmpValue::U64(u)),
        Value::String(s) => Ok(KeyCmpValue::String(s)),
        Value::Tuple(Tuple(values)) => {
            let keys = values.into_iter()
                .map(value_to_key_cmp)
                .collect::<Result<Vec<_>>>()?;
            Ok(KeyCmpValue::Tuple(eure_value::value::Tuple(keys)))
        }
        Value::Unit => Ok(KeyCmpValue::Unit),
        _ => Err(Error::InvalidType(format!("cannot use {value:?} as map key"))),
    }
}