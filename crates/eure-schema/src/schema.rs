//! Schema representation types for EURE documents

use crate::{SchemaError, identifiers};
use eure_tree::tree::InputSpan;
use eure_value::identifier::Identifier;
use eure_value::value::{KeyCmpValue, EurePath, PathSegment};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::str::FromStr;

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

    // Code type (for both inline/named code and code blocks)
    Code(Option<String>), // language identifier is optional

    // Collection types
    Array(Box<Type>),
    Object(ObjectSchema),

    // Tuple type - fixed-length array with specific types for each position
    Tuple(Vec<Type>),

    // Union types
    Union(Vec<Type>),        // Untagged union
    Variants(VariantSchema), // Tagged union with $variant

    // Type reference
    TypeRef(Identifier), // Reference to $types.name
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

/// How variants are represented in EURE documents.
///
/// This enum defines the different strategies for encoding variant types (algebraic data types/tagged unions)
/// in EURE documents. Each representation has different trade-offs in terms of clarity, conciseness, and compatibility.
#[derive(Debug, Clone, PartialEq)]
pub enum VariantRepr {
    /// Tagged representation (default): The variant is identified by an object key or `$variant` extension.
    ///
    /// # Examples
    /// Using object key:
    /// ```eure
    /// @ command {
    ///   echo {
    ///     message = "Hello"
    ///   }
    /// }
    /// ```
    ///
    /// Using `$variant` extension:
    /// ```eure
    /// @ command {
    ///   $variant: echo
    ///   message = "Hello"
    /// }
    /// ```
    ///
    /// # Characteristics
    /// - Clear and explicit variant identification
    /// - Supports both nested and flat field structures
    /// - Most idiomatic for EURE
    Tagged,

    /// Untagged representation: The variant is determined by structural matching.
    ///
    /// # Example
    /// ```eure
    /// @ value {
    ///   text = "Hello"    # Matches 'text' variant by structure
    ///   lang = "en"
    /// }
    /// ```
    ///
    /// # Characteristics
    /// - Most concise representation
    /// - No explicit variant tag needed
    /// - Can be ambiguous if variants have similar structures
    /// - Performance cost: must try each variant until one matches
    Untagged,

    /// Internally tagged: The variant is identified by a field within the object.
    ///
    /// # Example
    /// With tag field "type":
    /// ```eure
    /// @ event {
    ///   type = "click"    # This field determines the variant
    ///   x = 100
    ///   y = 200
    /// }
    /// ```
    ///
    /// # Characteristics
    /// - Compatible with many JSON APIs
    /// - Tag is part of the variant data
    /// - All variant fields at same level as tag
    InternallyTagged {
        /// The field name that contains the variant identifier
        tag: KeyCmpValue,
    },

    /// Adjacently tagged: Tag and content are in separate fields.
    ///
    /// # Example
    /// With tag="type" and content="data":
    /// ```eure
    /// @ message {
    ///   type = "text"      # Tag field
    ///   data = {           # Content field
    ///     content = "Hello"
    ///     formatted = true
    ///   }
    /// }
    /// ```
    ///
    /// # Characteristics
    /// - Clear separation between metadata and content
    /// - Compatible with envelope patterns
    /// - More verbose than other representations
    AdjacentlyTagged {
        /// The field name that contains the variant identifier
        tag: KeyCmpValue,
        /// The field name that contains the variant content
        content: KeyCmpValue,
    },
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
}

impl FromStr for RenameRule {
    type Err = SchemaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "camelCase" => Ok(Self::CamelCase),
            "snake_case" => Ok(Self::SnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "PascalCase" => Ok(Self::PascalCase),
            "UPPERCASE" => Ok(Self::Uppercase),
            "lowercase" => Ok(Self::Lowercase),
            _ => Err(SchemaError::InvalidRenameRule(s.to_string())),
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
    /// Cascade types mapped by their definition path
    /// A cascade at path [a, b] applies to all descendants of a.b
    pub cascade_types: HashMap<EurePath, Type>,
    /// Global serde options
    pub serde_options: SerdeOptions,
    /// Reference to external schema (from $schema key)
    pub schema_ref: Option<String>,
}

impl Type {
    /// Parse a type from path segments (preserves type information)
    pub fn from_path_segments(segments: &[PathSegment]) -> Option<Self> {
        if segments.is_empty() {
            return None;
        }

        match &segments[0] {
            // Check for type references: $types.TypeName
            PathSegment::Extension(ext) if ext.as_ref() == "types" => {
                if segments.len() >= 2
                    && let PathSegment::Ident(type_name) = &segments[1]
                {
                    return Some(Type::TypeRef(type_name.clone()));
                }
                None
            }
            // Check for primitive types and code types
            PathSegment::Ident(ident) => {
                match ident.as_ref() {
                    "string" => Some(Type::String),
                    "number" => Some(Type::Number),
                    "boolean" => Some(Type::Boolean),
                    "null" => Some(Type::Null),
                    "any" => Some(Type::Any),
                    "path" => Some(Type::Path),
                    "array" => Some(Type::Array(Box::new(Type::Any))),
                    "object" => Some(Type::Object(ObjectSchema::default())),
                    "code" => {
                        // Check if there's a language specifier
                        if segments.len() >= 2 {
                            if let PathSegment::Ident(lang) = &segments[1] {
                                Some(Type::Code(Some(lang.to_string())))
                            } else {
                                Some(Type::Code(None))
                            }
                        } else {
                            Some(Type::Code(None))
                        }
                    }
                    name => {
                        // If it starts with uppercase, treat it as a type reference
                        // This allows .Action to be shorthand for .$types.Action
                        if name
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false)
                        {
                            Some(Type::TypeRef(ident.clone()))
                        } else {
                            None
                        }
                    }
                }
            }
            // Handle path segments that are values (like .null, .true, .false)
            PathSegment::Value(val) => match val {
                KeyCmpValue::Null => Some(Type::Null),
                KeyCmpValue::Bool(_) => Some(Type::Boolean),
                KeyCmpValue::I64(_) | KeyCmpValue::U64(_) => Some(Type::Number),
                KeyCmpValue::String(_) => Some(Type::String),
                _ => None,
            },
            _ => None,
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
                type_expr: Type::TypeRef(
                    Identifier::from_str(name)
                        .unwrap_or_else(|_| identifiers::UNKNOWN_CAPS.clone()),
                ),
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
