//! Synthesized type definitions
//!
//! These types represent inferred types from Eure document values,
//! without the constraint information found in schema types.

use eure_document::identifier::Identifier;
use std::collections::HashMap;
use std::fmt;

/// A synthesized type inferred from document values.
///
/// Unlike `SchemaNodeContent`, this type does not include validation
/// constraints (min/max length, patterns, etc.) - it represents only
/// the structural type of the data.
#[derive(Debug, Clone, PartialEq)]
pub enum SynthType {
    // === Primitives ===
    /// Null value
    Null,

    /// Boolean value
    Boolean,

    /// Integer value (arbitrary precision)
    Integer,

    /// Floating-point value
    Float,

    /// Text value with optional language tag
    ///
    /// - `None` - implicit/unknown language (from `` `...` ``)
    /// - `Some("plaintext")` - plaintext (from `"..."`)
    /// - `Some("rust")` - language-tagged (from `` rust`...` ``)
    Text(Option<String>),

    // === Compounds ===
    /// Homogeneous array type
    Array(Box<SynthType>),

    /// Tuple type (fixed-length, heterogeneous)
    Tuple(Vec<SynthType>),

    /// Record type (fixed named fields)
    Record(SynthRecord),

    // === Special ===
    /// Union of types (structural, not tagged)
    ///
    /// Invariants:
    /// - At least 2 variants
    /// - No nested unions (flattened)
    /// - No duplicate variants
    Union(SynthUnion),

    /// Top type - accepts any value
    ///
    /// Used for:
    /// - Empty arrays: `[]` has type `Array<Any>`
    /// - Unknown types
    Any,

    /// Bottom type - no values
    ///
    /// Used for contradictions (rare in practice)
    Never,

    /// Unfilled placeholder
    ///
    /// Holes are absorbed during unification:
    /// `unify(Integer, Hole) = Integer`
    Hole(Option<Identifier>),
}

/// A record type with named fields
#[derive(Debug, Clone, PartialEq)]
pub struct SynthRecord {
    /// Field name to field definition
    pub fields: HashMap<String, SynthField>,
}

/// A field in a record type
#[derive(Debug, Clone, PartialEq)]
pub struct SynthField {
    /// The type of the field
    pub ty: SynthType,

    /// Whether the field is optional
    ///
    /// Note: In synthesized types, optionality comes from unifying
    /// records where some have the field and others don't.
    pub optional: bool,
}

/// A structural union of types
///
/// Unlike schema unions, these are anonymous/structural and don't
/// have named variants.
#[derive(Debug, Clone, PartialEq)]
pub struct SynthUnion {
    /// The variant types in this union
    ///
    /// Invariants:
    /// - At least 2 elements
    /// - No `SynthType::Union` elements (flattened)
    /// - No duplicates
    pub variants: Vec<SynthType>,
}

// === Constructors ===

impl SynthRecord {
    /// Create an empty record
    pub fn empty() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Create a record from field definitions
    pub fn new(fields: impl IntoIterator<Item = (String, SynthField)>) -> Self {
        Self {
            fields: fields.into_iter().collect(),
        }
    }
}

impl SynthField {
    /// Create a required field
    pub fn required(ty: SynthType) -> Self {
        Self {
            ty,
            optional: false,
        }
    }

    /// Create an optional field
    pub fn optional(ty: SynthType) -> Self {
        Self { ty, optional: true }
    }
}

impl SynthUnion {
    /// Create a union from variants, normalizing as needed
    ///
    /// This function:
    /// - Flattens nested unions
    /// - Removes duplicates
    /// - Returns single type if only one variant remains
    /// - Returns `Never` for empty input
    pub fn from_variants(variants: impl IntoIterator<Item = SynthType>) -> SynthType {
        let mut flat: Vec<SynthType> = Vec::new();

        for variant in variants {
            match variant {
                // Flatten nested unions
                SynthType::Union(inner) => {
                    for v in inner.variants {
                        if !flat.contains(&v) {
                            flat.push(v);
                        }
                    }
                }
                // Skip Never (identity for union)
                SynthType::Never => {}
                // Skip Holes (absorbed)
                SynthType::Hole(_) => {}
                // Add if not duplicate
                other => {
                    if !flat.contains(&other) {
                        flat.push(other);
                    }
                }
            }
        }

        match flat.len() {
            0 => SynthType::Never,
            1 => flat.pop().unwrap(),
            _ => SynthType::Union(SynthUnion { variants: flat }),
        }
    }
}

// === Display implementations ===

impl fmt::Display for SynthType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SynthType::Null => write!(f, "null"),
            SynthType::Boolean => write!(f, "boolean"),
            SynthType::Integer => write!(f, "integer"),
            SynthType::Float => write!(f, "float"),
            SynthType::Text(None) => write!(f, "text"),
            SynthType::Text(Some(lang)) => write!(f, "text.{}", lang),
            SynthType::Array(inner) => write!(f, "[{}]", inner),
            SynthType::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
            SynthType::Record(rec) => write!(f, "{}", rec),
            SynthType::Union(union) => write!(f, "{}", union),
            SynthType::Any => write!(f, "any"),
            SynthType::Never => write!(f, "never"),
            SynthType::Hole(None) => write!(f, "!"),
            SynthType::Hole(Some(id)) => write!(f, "!{}", id),
        }
    }
}

impl fmt::Display for SynthRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for (name, field) in &self.fields {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{}", name)?;
            if field.optional {
                write!(f, "?")?;
            }
            write!(f, ": {}", field.ty)?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for SynthUnion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, variant) in self.variants.iter().enumerate() {
            if i > 0 {
                write!(f, " | ")?;
            }
            write!(f, "{}", variant)?;
        }
        Ok(())
    }
}

// === Type predicates ===

impl SynthType {
    /// Check if this is a primitive type
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            SynthType::Null
                | SynthType::Boolean
                | SynthType::Integer
                | SynthType::Float
                | SynthType::Text(_)
        )
    }

    /// Check if this is a compound type
    pub fn is_compound(&self) -> bool {
        matches!(
            self,
            SynthType::Array(_) | SynthType::Tuple(_) | SynthType::Record(_)
        )
    }

    /// Check if this type contains any holes
    pub fn has_holes(&self) -> bool {
        match self {
            SynthType::Hole(_) => true,
            SynthType::Array(inner) => inner.has_holes(),
            SynthType::Tuple(elems) => elems.iter().any(|e| e.has_holes()),
            SynthType::Record(rec) => rec.fields.values().any(|f| f.ty.has_holes()),
            SynthType::Union(union) => union.variants.iter().any(|v| v.has_holes()),
            _ => false,
        }
    }

    /// Check if this type is complete (no holes, no Any, no Never)
    pub fn is_complete(&self) -> bool {
        match self {
            SynthType::Hole(_) | SynthType::Any | SynthType::Never => false,
            SynthType::Array(inner) => inner.is_complete(),
            SynthType::Tuple(elems) => elems.iter().all(|e| e.is_complete()),
            SynthType::Record(rec) => rec.fields.values().all(|f| f.ty.is_complete()),
            SynthType::Union(union) => union.variants.iter().all(|v| v.is_complete()),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_flattening() {
        // Union of unions should flatten
        let inner = SynthUnion::from_variants([SynthType::Integer, SynthType::Boolean]);
        let outer = SynthUnion::from_variants([inner, SynthType::Text(None)]);

        assert_eq!(
            outer,
            SynthType::Union(SynthUnion {
                variants: vec![
                    SynthType::Integer,
                    SynthType::Boolean,
                    SynthType::Text(None)
                ]
            })
        );
    }

    #[test]
    fn test_union_dedup() {
        let union =
            SynthUnion::from_variants([SynthType::Integer, SynthType::Integer, SynthType::Boolean]);

        assert_eq!(
            union,
            SynthType::Union(SynthUnion {
                variants: vec![SynthType::Integer, SynthType::Boolean]
            })
        );
    }

    #[test]
    fn test_union_single_collapses() {
        let union = SynthUnion::from_variants([SynthType::Integer]);
        assert_eq!(union, SynthType::Integer);
    }

    #[test]
    fn test_union_absorbs_holes() {
        let union = SynthUnion::from_variants([SynthType::Integer, SynthType::Hole(None)]);
        assert_eq!(union, SynthType::Integer);
    }

    #[test]
    fn test_union_absorbs_never() {
        let union = SynthUnion::from_variants([SynthType::Integer, SynthType::Never]);
        assert_eq!(union, SynthType::Integer);
    }

    #[test]
    fn test_display() {
        assert_eq!(SynthType::Integer.to_string(), "integer");
        assert_eq!(
            SynthType::Text(Some("rust".to_string())).to_string(),
            "text.rust"
        );
        assert_eq!(
            SynthType::Array(Box::new(SynthType::Integer)).to_string(),
            "[integer]"
        );
    }

    #[test]
    fn test_has_holes() {
        assert!(!SynthType::Integer.has_holes());
        assert!(SynthType::Hole(None).has_holes());
        assert!(SynthType::Array(Box::new(SynthType::Hole(None))).has_holes());
    }
}
