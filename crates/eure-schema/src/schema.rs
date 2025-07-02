//! Schema representation types for EURE documents

use eure_tree::tree::InputSpan;
use eure_value::value::KeyCmpValue;
use indexmap::IndexMap;

/// Core type representation in EURE schema
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Primitive types
    String,
    Number,
    Boolean,
    Null,
    Any,
    Path,

    // Typed strings
    TypedString(TypedStringKind),

    // Code blocks
    Code(String), // language identifier

    // Collection types
    Array(Box<Type>),
    Object(ObjectSchema),

    // Union types
    Union(Vec<Type>),        // Untagged union
    Variants(VariantSchema), // Tagged union with $variant

    // Type reference
    TypeRef(KeyCmpValue), // Reference to $types.name

    // Special types
    CascadeType(Box<Type>), // Type that cascades to descendants
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedStringKind {
    Email,
    Url,
    Uuid,
    Date,
    DateTime,
    Regex,
    Semver,
}

/// Schema for object types
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObjectSchema {
    pub fields: IndexMap<KeyCmpValue, FieldSchema>,
    pub additional_properties: Option<Box<Type>>,
}

/// Schema for a single field
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSchema {
    pub type_expr: Type,
    pub optional: bool,
    pub constraints: Constraints,
    pub preferences: Preferences,
    pub serde: SerdeOptions,
    pub span: Option<InputSpan>,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
}

impl Default for FieldSchema {
    fn default() -> Self {
        Self {
            type_expr: Type::Any,
            optional: false,
            constraints: Constraints::default(),
            preferences: Preferences::default(),
            serde: SerdeOptions::default(),
            span: None,
            default_value: None,
            description: None,
        }
    }
}

/// Schema for variant types (tagged unions)
#[derive(Debug, Clone, PartialEq)]
pub struct VariantSchema {
    pub variants: IndexMap<KeyCmpValue, ObjectSchema>,
    pub representation: VariantRepr,
}

/// How variants are represented
#[derive(Debug, Clone, PartialEq)]
pub enum VariantRepr {
    /// Default: uses $variant field as discriminator
    Tagged,
    /// No discriminator field
    Untagged,
    /// Custom tag field name
    InternallyTagged { tag: KeyCmpValue },
    /// Separate tag and content fields
    AdjacentlyTagged { tag: KeyCmpValue, content: KeyCmpValue },
}

impl Default for VariantRepr {
    fn default() -> Self {
        Self::Tagged
    }
}

/// Constraints that can be applied to types
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Constraints {
    // String constraints
    pub length: Option<(Option<usize>, Option<usize>)>, // (min, max)
    pub pattern: Option<String>,                        // Regex pattern

    // Number constraints
    pub range: Option<(Option<f64>, Option<f64>)>, // (min, max) inclusive
    pub exclusive_min: Option<f64>,
    pub exclusive_max: Option<f64>,

    // Array constraints
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub unique: Option<bool>,
    pub contains: Option<serde_json::Value>,
}

/// Preferences for how values should be represented
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Preferences {
    /// Prefer @ section syntax
    pub section: Option<bool>,
    /// Prefer @ field[] array append syntax
    pub array: Option<bool>,
}

/// Serialization/deserialization options
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SerdeOptions {
    /// Rename this field during serialization
    pub rename: Option<String>,
    /// Apply naming convention to all fields
    pub rename_all: Option<RenameRule>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenameRule {
    CamelCase,
    SnakeCase,
    KebabCase,
    PascalCase,
    Lowercase,
    Uppercase,
}

impl RenameRule {
    pub fn apply(&self, name: &str) -> String {
        use crate::utils::*;
        match self {
            Self::CamelCase => to_camel_case(name),
            Self::SnakeCase => to_snake_case(name),
            Self::KebabCase => to_kebab_case(name),
            Self::PascalCase => to_pascal_case(name),
            Self::Lowercase => name.to_lowercase(),
            Self::Uppercase => name.to_uppercase(),
        }
    }
    
    /// Parse a rename rule from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "camelCase" => Some(Self::CamelCase),
            "snake_case" => Some(Self::SnakeCase),
            "kebab-case" => Some(Self::KebabCase),
            "PascalCase" => Some(Self::PascalCase),
            "UPPERCASE" => Some(Self::Uppercase),
            "lowercase" => Some(Self::Lowercase),
            _ => None,
        }
    }
}

/// Complete schema for a document
#[derive(Debug, Clone, Default)]
pub struct DocumentSchema {
    /// Type definitions in $types namespace
    pub types: IndexMap<KeyCmpValue, FieldSchema>,
    /// Schema for root object
    pub root: ObjectSchema,
    /// Type that cascades to all descendants
    pub cascade_type: Option<Type>,
    /// Global serde options
    pub serde_options: SerdeOptions,
    /// Reference to external schema (from $schema key)
    pub schema_ref: Option<String>,
}

/// Result of schema extraction
#[derive(Debug)]
pub struct ExtractedSchema {
    /// The extracted document schema
    pub document_schema: DocumentSchema,
    /// Whether this is a pure schema document (no data, only definitions)
    pub is_pure_schema: bool,
}


impl Type {
    /// Parse a type from a path string (e.g., ".string", ".$types.username")
    pub fn from_path(path: &str) -> Option<Self> {
        let path = path.strip_prefix('.')?;

        // Check for type references
        if let Some(path) = path.strip_prefix("$types.") {
            return Some(Type::TypeRef(KeyCmpValue::String(path.to_string())));
        }

        // Check primitive types
        match path {
            "string" => Some(Type::String),
            "number" => Some(Type::Number),
            "boolean" => Some(Type::Boolean),
            "null" => Some(Type::Null),
            "any" => Some(Type::Any),
            "path" => Some(Type::Path),
            "array" => Some(Type::Array(Box::new(Type::Any))),
            "object" => Some(Type::Object(ObjectSchema::default())),
            _ => {
                // Check typed strings
                if let Some(path) = path.strip_prefix("typed-string.") {
                    let kind = match path {
                        "email" => TypedStringKind::Email,
                        "url" => TypedStringKind::Url,
                        "uuid" => TypedStringKind::Uuid,
                        "date" => TypedStringKind::Date,
                        "datetime" => TypedStringKind::DateTime,
                        "regex" => TypedStringKind::Regex,
                        "semver" => TypedStringKind::Semver,
                        _ => return None,
                    };
                    Some(Type::TypedString(kind))
                } else if let Some(path) = path.strip_prefix("code.") {
                    Some(Type::Code(path.to_string()))
                } else {
                    // If it starts with uppercase, treat it as a type reference
                    // This allows .Action to be shorthand for .$types.Action
                    if path.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        Some(Type::TypeRef(KeyCmpValue::String(path.to_string())))
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Check if a type is compatible with another (for union type checking)
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Any, _) | (_, Type::Any) => true,
            (Type::Union(types), other) => types.iter().any(|t| t.is_compatible_with(other)),
            (other, Type::Union(types)) => types.iter().any(|t| other.is_compatible_with(t)),
            (a, b) => a == b,
        }
    }
}

/// Trait for types that can generate their own EURE schema
pub trait ToEureSchema {
    /// Generate the EURE schema for this type
    fn eure_schema() -> FieldSchema;
    
    /// Optional: Return the type name for named types
    fn type_name() -> Option<&'static str> {
        None
    }
    
    /// Generate schema for use as a field type (may return TypeRef to prevent recursion)
    fn eure_field_schema() -> FieldSchema {
        // By default, check if this is a named type and return a TypeRef
        if let Some(name) = Self::type_name() {
            FieldSchema {
                type_expr: Type::TypeRef(KeyCmpValue::String(name.to_string())),
                optional: false,
                constraints: Default::default(),
                preferences: Default::default(),
                serde: Default::default(),
                span: None,
                default_value: None,
                description: None,
            }
        } else {
            Self::eure_schema()
        }
    }
}
