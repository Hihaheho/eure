//! Builder pattern for constructing schemas programmatically

use crate::{
    FieldSchema, Type, ObjectSchema, VariantSchema, VariantRepr, 
    Constraints, Preferences, SerdeOptions, TypedStringKind
};
use indexmap::IndexMap;

/// Builder for creating FieldSchema instances
#[derive(Default)]
pub struct FieldSchemaBuilder {
    type_expr: Option<Type>,
    optional: bool,
    constraints: Constraints,
    preferences: Preferences,
    serde: SerdeOptions,
    default_value: Option<serde_json::Value>,
    description: Option<String>,
}

impl FieldSchemaBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the type expression
    pub fn type_expr(mut self, type_expr: Type) -> Self {
        self.type_expr = Some(type_expr);
        self
    }
    
    /// Set the field as optional
    pub fn optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }
    
    /// Set string length constraints
    pub fn length(mut self, min: Option<usize>, max: Option<usize>) -> Self {
        self.constraints.length = Some((min, max));
        self
    }
    
    /// Set regex pattern constraint
    pub fn pattern(mut self, pattern: impl Into<String>) -> Self {
        self.constraints.pattern = Some(pattern.into());
        self
    }
    
    /// Set numeric range constraints
    pub fn range(mut self, min: Option<f64>, max: Option<f64>) -> Self {
        self.constraints.range = Some((min, max));
        self
    }
    
    /// Set minimum items for arrays
    pub fn min_items(mut self, min: usize) -> Self {
        self.constraints.min_items = Some(min);
        self
    }
    
    /// Set maximum items for arrays
    pub fn max_items(mut self, max: usize) -> Self {
        self.constraints.max_items = Some(max);
        self
    }
    
    /// Prefer section syntax
    pub fn prefer_section(mut self, prefer: bool) -> Self {
        self.preferences.section = Some(prefer);
        self
    }
    
    /// Set serde rename
    pub fn rename(mut self, name: impl Into<String>) -> Self {
        self.serde.rename = Some(name.into());
        self
    }
    
    /// Set default value
    pub fn default_value(mut self, value: serde_json::Value) -> Self {
        self.default_value = Some(value);
        self
    }
    
    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    
    /// Build the FieldSchema
    pub fn build(self) -> Result<FieldSchema, &'static str> {
        let type_expr = self.type_expr.ok_or("type_expr is required")?;
        
        Ok(FieldSchema {
            type_expr,
            optional: self.optional,
            constraints: self.constraints,
            preferences: self.preferences,
            serde: self.serde,
            span: None,
            default_value: self.default_value,
            description: self.description,
        })
    }
}

/// Builder for creating Type instances
pub struct TypeBuilder;

impl TypeBuilder {
    /// Create a string type
    pub fn string() -> Type {
        Type::String
    }
    
    /// Create a number type
    pub fn number() -> Type {
        Type::Number
    }
    
    /// Create a boolean type
    pub fn boolean() -> Type {
        Type::Boolean
    }
    
    /// Create a null type
    pub fn null() -> Type {
        Type::Null
    }
    
    /// Create an any type
    pub fn any() -> Type {
        Type::Any
    }
    
    /// Create a path type
    pub fn path() -> Type {
        Type::Path
    }
    
    /// Create a typed string
    pub fn typed_string(kind: TypedStringKind) -> Type {
        Type::TypedString(kind)
    }
    
    /// Create an email type
    pub fn email() -> Type {
        Type::TypedString(TypedStringKind::Email)
    }
    
    /// Create a URL type
    pub fn url() -> Type {
        Type::TypedString(TypedStringKind::Url)
    }
    
    /// Create a UUID type
    pub fn uuid() -> Type {
        Type::TypedString(TypedStringKind::Uuid)
    }
    
    /// Create a date type
    pub fn date() -> Type {
        Type::TypedString(TypedStringKind::Date)
    }
    
    /// Create a datetime type
    pub fn datetime() -> Type {
        Type::TypedString(TypedStringKind::DateTime)
    }
    
    /// Create an array type
    pub fn array(item_type: Type) -> Type {
        Type::Array(Box::new(item_type))
    }
    
    /// Create an object type builder
    pub fn object() -> ObjectSchemaBuilder {
        ObjectSchemaBuilder::new()
    }
    
    /// Create a union type
    pub fn union(types: Vec<Type>) -> Type {
        Type::Union(types)
    }
    
    /// Create a variants type builder
    pub fn variants() -> VariantSchemaBuilder {
        VariantSchemaBuilder::new()
    }
    
    /// Create a type reference
    pub fn type_ref(name: impl Into<String>) -> Type {
        Type::TypeRef(name.into())
    }
    
    /// Create a cascade type
    pub fn cascade(inner: Type) -> Type {
        Type::CascadeType(Box::new(inner))
    }
}

/// Builder for ObjectSchema
#[derive(Default)]
pub struct ObjectSchemaBuilder {
    fields: IndexMap<String, FieldSchema>,
    additional_properties: Option<Box<Type>>,
}

impl ObjectSchemaBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a field
    pub fn field(mut self, name: impl Into<String>, schema: FieldSchema) -> Self {
        self.fields.insert(name.into(), schema);
        self
    }
    
    /// Add a field using a builder
    pub fn field_with<F>(mut self, name: impl Into<String>, f: F) -> Self 
    where
        F: FnOnce(FieldSchemaBuilder) -> FieldSchemaBuilder
    {
        let builder = f(FieldSchemaBuilder::new());
        if let Ok(schema) = builder.build() {
            self.fields.insert(name.into(), schema);
        }
        self
    }
    
    /// Set additional properties type
    pub fn additional_properties(mut self, type_expr: Type) -> Self {
        self.additional_properties = Some(Box::new(type_expr));
        self
    }
    
    /// Build the ObjectSchema
    pub fn build(self) -> ObjectSchema {
        ObjectSchema {
            fields: self.fields,
            additional_properties: self.additional_properties,
        }
    }
    
    /// Build and wrap in Type::Object
    pub fn build_type(self) -> Type {
        Type::Object(self.build())
    }
}

/// Builder for VariantSchema
#[derive(Default)]
pub struct VariantSchemaBuilder {
    variants: IndexMap<String, ObjectSchema>,
    representation: VariantRepr,
}

impl VariantSchemaBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a variant
    pub fn variant(mut self, name: impl Into<String>, schema: ObjectSchema) -> Self {
        self.variants.insert(name.into(), schema);
        self
    }
    
    /// Add a variant using a builder
    pub fn variant_with<F>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(ObjectSchemaBuilder) -> ObjectSchemaBuilder
    {
        let schema = f(ObjectSchemaBuilder::new()).build();
        self.variants.insert(name.into(), schema);
        self
    }
    
    /// Set as untagged
    pub fn untagged(mut self) -> Self {
        self.representation = VariantRepr::Untagged;
        self
    }
    
    /// Set as internally tagged
    pub fn internally_tagged(mut self, tag: impl Into<String>) -> Self {
        self.representation = VariantRepr::InternallyTagged { tag: tag.into() };
        self
    }
    
    /// Set as adjacently tagged
    pub fn adjacently_tagged(mut self, tag: impl Into<String>, content: impl Into<String>) -> Self {
        self.representation = VariantRepr::AdjacentlyTagged {
            tag: tag.into(),
            content: content.into(),
        };
        self
    }
    
    /// Build the VariantSchema
    pub fn build(self) -> VariantSchema {
        VariantSchema {
            variants: self.variants,
            representation: self.representation,
        }
    }
    
    /// Build and wrap in Type::Variants
    pub fn build_type(self) -> Type {
        Type::Variants(self.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_field_builder() {
        let field = FieldSchemaBuilder::new()
            .type_expr(TypeBuilder::string())
            .optional(true)
            .length(Some(3), Some(20))
            .pattern("^[a-z]+$")
            .build()
            .unwrap();
            
        assert_eq!(field.type_expr, Type::String);
        assert!(field.optional);
        assert_eq!(field.constraints.length, Some((Some(3), Some(20))));
        assert_eq!(field.constraints.pattern, Some("^[a-z]+$".to_string()));
    }
    
    #[test]
    fn test_object_builder() {
        let obj_type = TypeBuilder::object()
            .field_with("name", |f| f.type_expr(TypeBuilder::string()))
            .field_with("age", |f| f.type_expr(TypeBuilder::number()).optional(true))
            .field_with("email", |f| {
                f.type_expr(TypeBuilder::email())
                    .pattern(r"^[^@]+@[^@]+\.[^@]+$")
            })
            .build_type();
            
        if let Type::Object(schema) = obj_type {
            assert_eq!(schema.fields.len(), 3);
            assert!(schema.fields.contains_key("name"));
            assert!(schema.fields.contains_key("age"));
            assert!(schema.fields.contains_key("email"));
            assert!(schema.fields["age"].optional);
        } else {
            panic!("Expected object type");
        }
    }
    
    #[test]
    fn test_variant_builder() {
        let variant_type = TypeBuilder::variants()
            .variant_with("Success", |o| {
                o.field_with("data", |f| f.type_expr(TypeBuilder::any()))
            })
            .variant_with("Error", |o| {
                o.field_with("message", |f| f.type_expr(TypeBuilder::string()))
                 .field_with("code", |f| f.type_expr(TypeBuilder::number()))
            })
            .internally_tagged("type")
            .build_type();
            
        if let Type::Variants(schema) = variant_type {
            assert_eq!(schema.variants.len(), 2);
            assert!(schema.variants.contains_key("Success"));
            assert!(schema.variants.contains_key("Error"));
            assert_eq!(
                schema.representation, 
                VariantRepr::InternallyTagged { tag: "type".to_string() }
            );
        } else {
            panic!("Expected variants type");
        }
    }
}