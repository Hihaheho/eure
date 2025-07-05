//! Standard library implementations for ToEureSchema

use crate::{FieldSchema, ToEureSchema, Type, ObjectSchema};
use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use indexmap::IndexMap;

// Primitive types

impl ToEureSchema for bool {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::Boolean,
            ..Default::default()
        }
    }
}

impl ToEureSchema for String {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::String,
            ..Default::default()
        }
    }
}

impl ToEureSchema for &str {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::String,
            ..Default::default()
        }
    }
}

// Numeric types

macro_rules! impl_number {
    ($($t:ty),*) => {
        $(
            impl ToEureSchema for $t {
                fn eure_schema() -> FieldSchema {
                    FieldSchema {
                        type_expr: Type::Number,
                        ..Default::default()
                    }
                }
            }
        )*
    }
}

impl_number!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

impl ToEureSchema for char {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::String,
            ..Default::default()
        }
    }
}

// Option<T>

impl<T: ToEureSchema> ToEureSchema for Option<T> {
    fn eure_schema() -> FieldSchema {
        let mut schema = T::eure_field_schema();
        schema.optional = true;
        schema
    }
}

// Vec<T> and arrays

impl<T: ToEureSchema> ToEureSchema for Vec<T> {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::Array(Box::new(T::eure_field_schema().type_expr)),
            ..Default::default()
        }
    }
}

impl<T: ToEureSchema, const N: usize> ToEureSchema for [T; N] {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::Array(Box::new(T::eure_field_schema().type_expr)),
            ..Default::default()
        }
    }
}

impl<T: ToEureSchema> ToEureSchema for &[T] {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::Array(Box::new(T::eure_field_schema().type_expr)),
            ..Default::default()
        }
    }
}

// HashMap and BTreeMap

impl<K: ToEureSchema, V: ToEureSchema> ToEureSchema for HashMap<K, V> {
    fn eure_schema() -> FieldSchema {
        // For now, we represent maps as objects with additional properties
        FieldSchema {
            type_expr: Type::Object(ObjectSchema {
                fields: IndexMap::new(),
                additional_properties: Some(Box::new(V::eure_schema().type_expr)),
            }),
            ..Default::default()
        }
    }
}

impl<K: ToEureSchema, V: ToEureSchema> ToEureSchema for BTreeMap<K, V> {
    fn eure_schema() -> FieldSchema {
        // Same as HashMap
        FieldSchema {
            type_expr: Type::Object(ObjectSchema {
                fields: IndexMap::new(),
                additional_properties: Some(Box::new(V::eure_schema().type_expr)),
            }),
            ..Default::default()
        }
    }
}

// HashSet and BTreeSet

impl<T: ToEureSchema> ToEureSchema for HashSet<T> {
    fn eure_schema() -> FieldSchema {
        Vec::<T>::eure_schema()
    }
}

impl<T: ToEureSchema> ToEureSchema for BTreeSet<T> {
    fn eure_schema() -> FieldSchema {
        Vec::<T>::eure_schema()
    }
}

// Path types

impl ToEureSchema for PathBuf {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::String,
            ..Default::default()
        }
    }
}

impl ToEureSchema for &Path {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::String,
            ..Default::default()
        }
    }
}

// Unit type

impl ToEureSchema for () {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::Null,
            ..Default::default()
        }
    }
}

// Box<T>

impl<T: ToEureSchema> ToEureSchema for Box<T> {
    fn eure_schema() -> FieldSchema {
        T::eure_field_schema()
    }
}

// Rc and Arc

impl<T: ToEureSchema> ToEureSchema for std::rc::Rc<T> {
    fn eure_schema() -> FieldSchema {
        T::eure_field_schema()
    }
}

impl<T: ToEureSchema> ToEureSchema for std::sync::Arc<T> {
    fn eure_schema() -> FieldSchema {
        T::eure_field_schema()
    }
}

// Cow

impl<T: ToEureSchema + ToOwned + ?Sized> ToEureSchema for std::borrow::Cow<'_, T> 
where
    T::Owned: ToEureSchema,
{
    fn eure_schema() -> FieldSchema {
        T::Owned::eure_schema()
    }
}

// Result<T, E> - represented as a union

impl<T: ToEureSchema, E: ToEureSchema> ToEureSchema for Result<T, E> {
    fn eure_schema() -> FieldSchema {
        FieldSchema {
            type_expr: Type::Union(vec![
                T::eure_field_schema().type_expr,
                E::eure_field_schema().type_expr,
            ]),
            ..Default::default()
        }
    }
}

// Feature-gated implementations

#[cfg(feature = "chrono")]
mod chrono_impls {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, DateTime};
    
    impl ToEureSchema for NaiveDate {
        fn eure_schema() -> FieldSchema {
            FieldSchema {
                type_expr: Type::String, // date format
                ..Default::default()
            }
        }
    }
    
    impl ToEureSchema for NaiveDateTime {
        fn eure_schema() -> FieldSchema {
            FieldSchema {
                type_expr: Type::String, // datetime format
                ..Default::default()
            }
        }
    }
    
    impl<Tz: chrono::TimeZone> ToEureSchema for DateTime<Tz> {
        fn eure_schema() -> FieldSchema {
            FieldSchema {
                type_expr: Type::String, // datetime format
                ..Default::default()
            }
        }
    }
}

#[cfg(feature = "uuid")]
mod uuid_impls {
    use super::*;
    use uuid::Uuid;
    
    impl ToEureSchema for Uuid {
        fn eure_schema() -> FieldSchema {
            FieldSchema {
                type_expr: Type::String, // uuid format
                ..Default::default()
            }
        }
    }
}

#[cfg(feature = "url")]
mod url_impls {
    use super::*;
    use url::Url;
    
    impl ToEureSchema for Url {
        fn eure_schema() -> FieldSchema {
            FieldSchema {
                type_expr: Type::String, // url format
                ..Default::default()
            }
        }
    }
}

#[cfg(feature = "semver")]
mod semver_impls {
    use super::*;
    use semver::Version;
    
    impl ToEureSchema for Version {
        fn eure_schema() -> FieldSchema {
            FieldSchema {
                type_expr: Type::String, // semver format
                ..Default::default()
            }
        }
    }
}

