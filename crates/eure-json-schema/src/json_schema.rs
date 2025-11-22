//! JSON Schema representation as Rust ADT
//!
//! This module provides a strongly-typed representation of JSON Schema (Draft-07)
//! using Rust's algebraic data types. Each variant contains only the fields
//! relevant to that schema type, avoiding the "bag of optional fields" anti-pattern.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Common metadata fields for all schema types
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
}

/// JSON Schema root type
///
/// Uses `#[serde(untagged)]` to discriminate based on the presence of specific fields.
/// The order matters: more specific variants should come first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum JsonSchema {
    /// Boolean schema (true = allow all, false = deny all)
    Boolean(bool),

    /// Reference to another schema ($ref)
    Reference(ReferenceSchema),

    /// Enum constraint (fixed set of values)
    Enum(EnumSchema),

    /// Const constraint (single fixed value)
    Const(ConstSchema),

    /// AllOf composition
    AllOf(AllOfSchema),

    /// AnyOf composition
    AnyOf(AnyOfSchema),

    /// OneOf composition
    OneOf(OneOfSchema),

    /// Not composition
    Not(NotSchema),

    /// Typed schema with type-specific constraints
    Typed(TypedSchema),

    /// Generic schema (catch-all for schemas with metadata, definitions, or untyped constraints)
    /// This includes empty schemas and schemas with only metadata
    Generic(GenericSchema),
}

/// Reference schema ($ref)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSchema {
    #[serde(rename = "$ref")]
    pub reference: String,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Enum schema (fixed set of allowed values)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnumSchema {
    #[serde(rename = "enum")]
    pub values: Vec<serde_json::Value>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Const schema (single fixed value)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstSchema {
    #[serde(rename = "const")]
    pub value: serde_json::Value,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// AllOf schema (all schemas must match)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllOfSchema {
    #[serde(rename = "allOf")]
    pub schemas: Vec<JsonSchema>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// AnyOf schema (at least one schema must match)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnyOfSchema {
    #[serde(rename = "anyOf")]
    pub schemas: Vec<JsonSchema>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// OneOf schema (exactly one schema must match)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OneOfSchema {
    #[serde(rename = "oneOf")]
    pub schemas: Vec<JsonSchema>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Not schema (must not match the schema)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotSchema {
    pub not: Box<JsonSchema>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Typed schema (discriminated by "type" field)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum TypedSchema {
    #[serde(rename = "string")]
    String(StringSchema),

    #[serde(rename = "number")]
    Number(NumberSchema),

    #[serde(rename = "integer")]
    Integer(IntegerSchema),

    #[serde(rename = "boolean")]
    Boolean(BooleanSchema),

    #[serde(rename = "null")]
    Null(NullSchema),

    #[serde(rename = "array")]
    Array(ArraySchema),

    #[serde(rename = "object")]
    Object(ObjectSchema),
}

/// String type schema
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StringSchema {
    #[serde(rename = "minLength")]
    pub min_length: Option<u32>,

    #[serde(rename = "maxLength")]
    pub max_length: Option<u32>,

    pub pattern: Option<String>,

    pub format: Option<String>,

    pub default: Option<String>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Number type schema (floating point)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumberSchema {
    pub minimum: Option<f64>,

    pub maximum: Option<f64>,

    #[serde(rename = "exclusiveMinimum")]
    pub exclusive_minimum: Option<f64>,

    #[serde(rename = "exclusiveMaximum")]
    pub exclusive_maximum: Option<f64>,

    #[serde(rename = "multipleOf")]
    pub multiple_of: Option<f64>,

    pub default: Option<f64>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Integer type schema
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerSchema {
    pub minimum: Option<i64>,

    pub maximum: Option<i64>,

    #[serde(rename = "exclusiveMinimum")]
    pub exclusive_minimum: Option<i64>,

    #[serde(rename = "exclusiveMaximum")]
    pub exclusive_maximum: Option<i64>,

    #[serde(rename = "multipleOf")]
    pub multiple_of: Option<i64>,

    pub default: Option<i64>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Boolean type schema
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanSchema {
    pub default: Option<bool>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Null type schema
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NullSchema {
    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Array type schema
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArraySchema {
    pub items: Option<Box<JsonSchema>>,

    #[serde(rename = "minItems")]
    pub min_items: Option<u32>,

    #[serde(rename = "maxItems")]
    pub max_items: Option<u32>,

    #[serde(rename = "uniqueItems")]
    pub unique_items: Option<bool>,

    pub contains: Option<Box<JsonSchema>>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Object type schema (with explicit type: "object")
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSchema {
    pub properties: Option<IndexMap<String, JsonSchema>>,

    pub required: Option<Vec<String>>,

    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<AdditionalProperties>,

    // Note: patternProperties is not yet supported in this ADT
    // It would require more complex handling
    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

/// Additional properties policy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Bool(bool),
    Schema(Box<JsonSchema>),
}

/// Generic schema (catch-all)
/// This handles schemas without explicit type, including:
/// - Empty schemas {}
/// - Schemas with only metadata
/// - Schemas with object-specific fields but no type
/// - Schemas with definitions
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenericSchema {
    // Object-like fields (without explicit type)
    pub properties: Option<IndexMap<String, JsonSchema>>,
    pub required: Option<Vec<String>>,

    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<AdditionalProperties>,

    pub default: Option<serde_json::Value>,

    // Schema metadata
    #[serde(rename = "$schema")]
    pub schema: Option<String>,

    #[serde(rename = "$id")]
    pub id: Option<String>,

    #[serde(rename = "$defs")]
    pub defs: Option<IndexMap<String, JsonSchema>>,

    pub definitions: Option<IndexMap<String, JsonSchema>>,

    #[serde(flatten)]
    pub metadata: SchemaMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_boolean_schema_true() {
        let json = "true";
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Boolean(true) => {}
            _ => panic!("Expected Boolean(true)"),
        }
    }

    #[test]
    fn test_parse_boolean_schema_false() {
        let json = "false";
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Boolean(false) => {}
            _ => panic!("Expected Boolean(false)"),
        }
    }

    #[test]
    fn test_parse_simple_string_schema() {
        let json = r#"{"type": "string"}"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        assert_eq!(
            schema,
            JsonSchema::Typed(TypedSchema::String(StringSchema {
                min_length: None,
                max_length: None,
                pattern: None,
                format: None,
                default: None,
                metadata: SchemaMetadata::default(),
            }))
        );
    }

    #[test]
    fn test_parse_string_with_constraints() {
        let json = r#"{
            "type": "string",
            "minLength": 3,
            "maxLength": 20,
            "pattern": "^[a-z]+$"
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        assert_eq!(
            schema,
            JsonSchema::Typed(TypedSchema::String(StringSchema {
                min_length: Some(3),
                max_length: Some(20),
                pattern: Some("^[a-z]+$".to_string()),
                format: None,
                default: None,
                metadata: SchemaMetadata::default(),
            }))
        );
    }

    #[test]
    fn test_parse_integer_schema() {
        let json = r#"{
            "type": "integer",
            "minimum": 0,
            "maximum": 100
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Integer(i)) => {
                assert_eq!(i.minimum, Some(0));
                assert_eq!(i.maximum, Some(100));
            }
            _ => panic!("Expected Typed(Integer)"),
        }
    }

    #[test]
    fn test_parse_number_schema() {
        let json = r#"{
            "type": "number",
            "minimum": 0.0,
            "exclusiveMaximum": 1.0
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Number(n)) => {
                assert_eq!(n.minimum, Some(0.0));
                assert_eq!(n.exclusive_maximum, Some(1.0));
            }
            _ => panic!("Expected Typed(Number)"),
        }
    }

    #[test]
    fn test_parse_array_schema() {
        let json = r#"{
            "type": "array",
            "items": {"type": "string"},
            "minItems": 1,
            "uniqueItems": true
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Array(a)) => {
                assert_eq!(a.min_items, Some(1));
                assert_eq!(a.unique_items, Some(true));
                assert!(a.items.is_some());
            }
            _ => panic!("Expected Typed(Array)"),
        }
    }

    #[test]
    fn test_parse_object_with_properties() {
        let json = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Object(o)) => {
                assert!(o.properties.is_some());
                let props = o.properties.unwrap();
                assert_eq!(props.len(), 2);
                assert!(props.contains_key("name"));
                assert!(props.contains_key("age"));
                assert_eq!(o.required, Some(vec!["name".to_string()]));
            }
            _ => panic!("Expected Typed(Object)"),
        }
    }

    #[test]
    fn test_parse_object_with_additional_properties_bool() {
        let json = r#"{
            "type": "object",
            "additionalProperties": false
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Object(o)) => match o.additional_properties {
                Some(AdditionalProperties::Bool(false)) => {}
                _ => panic!("Expected additionalProperties: false"),
            },
            _ => panic!("Expected Typed(Object)"),
        }
    }

    #[test]
    fn test_parse_object_with_additional_properties_schema() {
        let json = r#"{
            "type": "object",
            "additionalProperties": {"type": "string"}
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Object(o)) => match o.additional_properties {
                Some(AdditionalProperties::Schema(_)) => {}
                _ => panic!("Expected additionalProperties with schema"),
            },
            _ => panic!("Expected Typed(Object)"),
        }
    }

    #[test]
    fn test_parse_reference_schema() {
        let json = r##"{"$ref": "#/definitions/User"}"##;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Reference(r) => {
                assert_eq!(r.reference, "#/definitions/User");
            }
            _ => panic!("Expected Reference"),
        }
    }

    #[test]
    fn test_parse_enum_schema() {
        let json = r#"{"enum": ["red", "green", "blue"]}"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Enum(e) => {
                assert_eq!(e.values.len(), 3);
            }
            _ => panic!("Expected Enum"),
        }
    }

    #[test]
    fn test_parse_const_schema() {
        let json = r#"{"const": "fixed-value"}"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Const(c) => {
                assert_eq!(c.value, serde_json::json!("fixed-value"));
            }
            _ => panic!("Expected Const"),
        }
    }

    #[test]
    fn test_parse_anyof_schema() {
        let json = r#"{
            "anyOf": [
                {"type": "string"},
                {"type": "number"}
            ]
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::AnyOf(a) => {
                assert_eq!(a.schemas.len(), 2);
            }
            _ => panic!("Expected AnyOf"),
        }
    }

    #[test]
    fn test_parse_allof_schema() {
        let json = r#"{
            "allOf": [
                {"type": "object"},
                {"properties": {"name": {"type": "string"}}}
            ]
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::AllOf(a) => {
                assert_eq!(a.schemas.len(), 2);
            }
            other => panic!("Expected AllOf, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_empty_schema() {
        let json = r#"{}"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Generic(g) => {
                // Empty schema is parsed as Generic with all fields None
                assert!(g.properties.is_none());
                assert!(g.metadata.title.is_none());
            }
            _ => panic!("Expected Generic"),
        }
    }

    #[test]
    fn test_parse_empty_schema_with_metadata() {
        let json = r#"{
            "title": "My Schema",
            "description": "A test schema"
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Generic(g) => {
                assert_eq!(g.metadata.title, Some("My Schema".to_string()));
                assert_eq!(g.metadata.description, Some("A test schema".to_string()));
            }
            _ => panic!("Expected Generic"),
        }
    }

    #[test]
    #[should_panic]
    fn test_reject_unknown_field_in_string_schema() {
        let json = r#"{
            "type": "string",
            "unknownField": "value"
        }"#;
        let _schema: JsonSchema = serde_json::from_str(json).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_reject_unknown_field_in_object_schema() {
        let json = r#"{
            "type": "object",
            "invalidField": true
        }"#;
        let _schema: JsonSchema = serde_json::from_str(json).unwrap();
    }

    #[test]
    fn test_roundtrip_string_schema() {
        let original = JsonSchema::Typed(TypedSchema::String(StringSchema {
            min_length: Some(5),
            max_length: Some(100),
            pattern: Some("^[A-Z]".to_string()),
            format: Some("email".to_string()),
            default: None,
            metadata: SchemaMetadata {
                title: Some("Email".to_string()),
                description: Some("User email address".to_string()),
            },
        }));

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: JsonSchema = serde_json::from_str(&json).unwrap();

        // Should be equal
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_roundtrip_object_schema() {
        let mut properties = IndexMap::new();
        properties.insert(
            "name".to_string(),
            JsonSchema::Typed(TypedSchema::String(StringSchema {
                min_length: Some(1),
                max_length: None,
                pattern: None,
                format: None,
                default: None,
                metadata: SchemaMetadata::default(),
            })),
        );
        properties.insert(
            "age".to_string(),
            JsonSchema::Typed(TypedSchema::Integer(IntegerSchema {
                minimum: Some(0),
                maximum: Some(150),
                exclusive_minimum: None,
                exclusive_maximum: None,
                multiple_of: None,
                default: None,
                metadata: SchemaMetadata::default(),
            })),
        );

        let original = JsonSchema::Typed(TypedSchema::Object(ObjectSchema {
            properties: Some(properties),
            required: Some(vec!["name".to_string()]),
            additional_properties: Some(AdditionalProperties::Bool(false)),
            metadata: SchemaMetadata::default(),
        }));

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: JsonSchema = serde_json::from_str(&json).unwrap();

        // Should be equal
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_roundtrip_anyof_schema() {
        let original = JsonSchema::AnyOf(AnyOfSchema {
            schemas: vec![
                JsonSchema::Typed(TypedSchema::String(StringSchema::default())),
                JsonSchema::Typed(TypedSchema::Integer(IntegerSchema::default())),
            ],
            metadata: SchemaMetadata::default(),
        });

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: JsonSchema = serde_json::from_str(&json).unwrap();

        // Should be equal
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_parse_boolean_type_schema() {
        let json = r#"{"type": "boolean"}"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Boolean(_)) => {}
            _ => panic!("Expected Typed(Boolean)"),
        }
    }

    #[test]
    fn test_parse_null_type_schema() {
        let json = r#"{"type": "null"}"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Null(_)) => {}
            _ => panic!("Expected Typed(Null)"),
        }
    }

    #[test]
    fn test_parse_oneof_schema() {
        let json = r#"{
            "oneOf": [
                {"type": "string"},
                {"type": "number"}
            ]
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::OneOf(o) => {
                assert_eq!(o.schemas.len(), 2);
            }
            _ => panic!("Expected OneOf"),
        }
    }

    #[test]
    fn test_parse_not_schema() {
        let json = r#"{
            "not": {"type": "string"}
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Not(n) => {
                assert!(matches!(*n.not, JsonSchema::Typed(TypedSchema::String(_))));
            }
            _ => panic!("Expected Not"),
        }
    }

    #[test]
    fn test_parse_generic_schema_with_properties() {
        let json = r#"{
            "properties": {
                "name": {"type": "string"}
            }
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Generic(g) => {
                assert!(g.properties.is_some());
                let props = g.properties.unwrap();
                assert_eq!(props.len(), 1);
                assert!(props.contains_key("name"));
            }
            _ => panic!("Expected Generic"),
        }
    }

    #[test]
    fn test_parse_nested_allof() {
        let json = r#"{
            "allOf": [
                {
                    "allOf": [
                        {"type": "object"},
                        {"properties": {"id": {"type": "string"}}}
                    ]
                },
                {"properties": {"name": {"type": "string"}}}
            ]
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::AllOf(a) => {
                assert_eq!(a.schemas.len(), 2);
                // First element should also be AllOf
                assert!(matches!(a.schemas[0], JsonSchema::AllOf(_)));
            }
            _ => panic!("Expected AllOf"),
        }
    }

    #[test]
    fn test_roundtrip_with_metadata() {
        let original = JsonSchema::Typed(TypedSchema::Integer(IntegerSchema {
            minimum: Some(0),
            maximum: Some(100),
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
            default: Some(50),
            metadata: SchemaMetadata {
                title: Some("Age".to_string()),
                description: Some("User's age in years".to_string()),
            },
        }));

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: JsonSchema = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_parse_array_with_contains() {
        let json = r#"{
            "type": "array",
            "contains": {"type": "string"}
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Array(a)) => {
                assert!(a.contains.is_some());
            }
            _ => panic!("Expected Typed(Array)"),
        }
    }

    #[test]
    fn test_parse_integer_with_multiple_of() {
        let json = r#"{
            "type": "integer",
            "multipleOf": 5
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::Integer(i)) => {
                assert_eq!(i.multiple_of, Some(5));
            }
            _ => panic!("Expected Typed(Integer)"),
        }
    }

    #[test]
    fn test_parse_string_with_format() {
        let json = r#"{
            "type": "string",
            "format": "email"
        }"#;
        let schema: JsonSchema = serde_json::from_str(json).unwrap();
        match schema {
            JsonSchema::Typed(TypedSchema::String(s)) => {
                assert_eq!(s.format, Some("email".to_string()));
            }
            _ => panic!("Expected Typed(String)"),
        }
    }

    #[test]
    fn test_roundtrip_reference_with_metadata() {
        let original = JsonSchema::Reference(ReferenceSchema {
            reference: "#/definitions/User".to_string(),
            metadata: SchemaMetadata {
                title: Some("User Reference".to_string()),
                description: Some("Reference to User type".to_string()),
            },
        });

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: JsonSchema = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }
}
